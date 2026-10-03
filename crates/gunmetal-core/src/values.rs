//! Typed values that every parser and the music model share
//! (SEC-MED-014).
//!
//! Identifiers and numeric fields from media files are validated into
//! these types when they are parsed, each with a documented range. A value
//! outside its range is dropped with a [`ValueError`] that says why, and
//! only these types reach SQL, provider URLs or the playback decision
//! engine. Divisors such as a sample rate are non-zero by type, so a zero
//! from a file never reaches a division.
//!
//! Text from a tag, a request or a provider arrives [`Untrusted`], and each
//! type's `parse` is the one way from it to the value (SEC-TM-031).
//!
//! The gain range is wide on purpose: the player clamps gain from tags to
//! −30 dB to +12 dB (SEC-MED-015), so a gain beyond that range is clamped
//! there rather than dropped here.

use std::fmt;
use std::num::{NonZeroU32, NonZeroU128};

use crate::time::days_in_month;
use crate::untrusted::Untrusted;

/// Which value a [`ValueError`] is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// A `MusicBrainz` identifier.
    Mbid,
    /// An International Standard Recording Code.
    Isrc,
    /// A sample rate.
    SampleRate,
    /// A channel count.
    Channels,
    /// A bit depth.
    BitDepth,
    /// A track or disc number.
    Number,
    /// A track or disc total.
    Total,
    /// The year of a date.
    Year,
    /// The month of a date.
    Month,
    /// The day of a date.
    Day,
    /// A `ReplayGain` or R128 gain.
    Gain,
    /// A `ReplayGain` peak.
    Peak,
    /// A duration.
    Duration,
}

/// Why a value was dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueError {
    /// The text is not written the way the value is written.
    Malformed {
        /// The value.
        field: Field,
    },
    /// A whole number outside the value's range.
    OutOfRange {
        /// The value.
        field: Field,
        /// The number given, saturated at `u64::MAX`.
        value: u64,
    },
    /// A gain or peak that is not a number, is infinite or is outside its
    /// range.
    Unusable {
        /// The value.
        field: Field,
    },
    /// A track or disc number greater than its total.
    AboveTotal {
        /// The number given.
        number: u16,
        /// The total given.
        total: u16,
    },
}

/// A `MusicBrainz` identifier: a UUID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Mbid([u8; 16]);

impl Mbid {
    /// Reads the hyphenated form, in either case. Braces, missing hyphens,
    /// whitespace and anything else are refused.
    ///
    /// # Errors
    ///
    /// [`ValueError::Malformed`] for anything else.
    pub fn parse(text: Untrusted<&str>) -> Result<Self, ValueError> {
        let text = text.into_inner();
        let malformed = ValueError::Malformed { field: Field::Mbid };
        if text.len() != 36 {
            return Err(malformed);
        }
        let mut value = 0_u128;
        for (index, octet) in text.bytes().enumerate() {
            match (index, octet) {
                (8 | 13 | 18 | 23, b'-') => {}
                (8 | 13 | 18 | 23, _) => return Err(malformed),
                (_, digit) => {
                    let nibble = char::from(digit).to_digit(16).ok_or(malformed)?;
                    // Thirty-two nibbles fill the 128 bits exactly, so
                    // the addition never saturates.
                    value = (value << 4).saturating_add(u128::from(nibble));
                }
            }
        }
        Ok(Self(value.to_be_bytes()))
    }
}

impl fmt::Display for Mbid {
    /// The hyphenated lower-case form.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = u128::from_be_bytes(self.0);
        write!(
            f,
            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            value >> 96,
            value >> 80 & 0xFFFF,
            value >> 64 & 0xFFFF,
            value >> 48 & 0xFFFF,
            value & 0xFFFF_FFFF_FFFF,
        )
    }
}

/// An International Standard Recording Code, such as `USS1Z9900001`: a
/// two-letter country code, a three-character registrant, two digits of
/// year and five of designation (ISO 3901).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Isrc(String);

impl Isrc {
    /// Reads the twelve-character code, in either case, or its hyphenated
    /// display form `CC-XXX-YY-NNNNN`.
    ///
    /// # Errors
    ///
    /// [`ValueError::Malformed`] for anything else.
    pub fn parse(text: Untrusted<&str>) -> Result<Self, ValueError> {
        let text = text.into_inner();
        let code: Vec<u8> = match text.as_bytes() {
            hyphenated @ [_, _, b'-', _, _, _, b'-', _, _, b'-', _, _, _, _, _] => hyphenated
                .iter()
                .filter(|&&octet| octet != b'-')
                .map(u8::to_ascii_uppercase)
                .collect(),
            plain => plain.iter().map(u8::to_ascii_uppercase).collect(),
        };
        let valid = code.len() == 12
            && code.iter().enumerate().all(|(index, octet)| match index {
                0 | 1 => octet.is_ascii_uppercase(),
                2..=4 => octet.is_ascii_alphanumeric(),
                _ => octet.is_ascii_digit(),
            });
        if valid {
            Ok(Self(code.into_iter().map(char::from).collect()))
        } else {
            Err(ValueError::Malformed { field: Field::Isrc })
        }
    }

    /// The code in upper case, without hyphens.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A whole number from 1 to `max`.
fn ranged(field: Field, value: u32, max: u32) -> Result<NonZeroU32, ValueError> {
    NonZeroU32::new(value)
        .filter(|_| value <= max)
        .ok_or(ValueError::OutOfRange {
            field,
            value: u64::from(value),
        })
}

/// A sample rate in hertz, from 1 to 768,000.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SampleRate(NonZeroU32);

impl SampleRate {
    /// The highest sample rate accepted.
    pub const MAX: u32 = 768_000;

    /// A sample rate of `hz`.
    ///
    /// # Errors
    ///
    /// [`ValueError::OutOfRange`] outside 1 to [`Self::MAX`].
    pub fn new(hz: u32) -> Result<Self, ValueError> {
        ranged(Field::SampleRate, hz, Self::MAX).map(Self)
    }

    /// The rate, never zero.
    #[must_use]
    pub const fn hz(self) -> NonZeroU32 {
        self.0
    }
}

/// A channel count, from 1 to 255.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Channels(NonZeroU32);

impl Channels {
    /// The most channels accepted.
    pub const MAX: u32 = 255;

    /// A count of `count` channels.
    ///
    /// # Errors
    ///
    /// [`ValueError::OutOfRange`] outside 1 to [`Self::MAX`].
    pub fn new(count: u32) -> Result<Self, ValueError> {
        ranged(Field::Channels, count, Self::MAX).map(Self)
    }

    /// The count, never zero.
    #[must_use]
    pub const fn get(self) -> NonZeroU32 {
        self.0
    }
}

/// Bits per sample, from 1 to 64.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BitDepth(NonZeroU32);

impl BitDepth {
    /// The deepest sample accepted.
    pub const MAX: u32 = 64;

    /// A depth of `bits` bits.
    ///
    /// # Errors
    ///
    /// [`ValueError::OutOfRange`] outside 1 to [`Self::MAX`].
    pub fn new(bits: u32) -> Result<Self, ValueError> {
        ranged(Field::BitDepth, bits, Self::MAX).map(Self)
    }

    /// The depth, never zero.
    #[must_use]
    pub const fn get(self) -> NonZeroU32 {
        self.0
    }
}

/// The value of a non-empty run of ASCII decimal digits, saturated at
/// `u64::MAX`, or `None` for anything else.
fn digits(text: &str) -> Option<u64> {
    if text.is_empty() {
        return None;
    }
    text.bytes().try_fold(0_u64, |value, octet| {
        char::from(octet)
            .to_digit(10)
            .map(|digit| value.saturating_mul(10).saturating_add(u64::from(digit)))
    })
}

/// A track or disc number, with the total when it is known. Both are from
/// 1 to 9,999, and the number is at most the total.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NumberOf {
    number: u16,
    total: Option<u16>,
}

impl NumberOf {
    /// The highest number or total accepted.
    pub const MAX: u16 = 9_999;

    /// Number `number` of `total`.
    ///
    /// # Errors
    ///
    /// [`ValueError::OutOfRange`] for a number or total outside 1 to
    /// [`Self::MAX`]; [`ValueError::AboveTotal`] for a number above its
    /// total.
    pub fn new(number: u64, total: Option<u64>) -> Result<Self, ValueError> {
        let count = |field, value| {
            u16::try_from(value)
                .ok()
                .filter(|count| (1..=Self::MAX).contains(count))
                .ok_or(ValueError::OutOfRange { field, value })
        };
        let number = count(Field::Number, number)?;
        let total = total.map(|total| count(Field::Total, total)).transpose()?;
        match total {
            Some(total) if number > total => Err(ValueError::AboveTotal { number, total }),
            _ => Ok(Self { number, total }),
        }
    }

    /// Reads `3`, `3/12`, `3 / 12` or `03 of 12`.
    ///
    /// # Errors
    ///
    /// [`ValueError::Malformed`] for other text, and the errors of
    /// [`Self::new`].
    pub fn parse(text: Untrusted<&str>) -> Result<Self, ValueError> {
        let text = text.into_inner();
        let lower = text.trim().to_ascii_lowercase();
        let (number, total) = match lower.split_once('/').or_else(|| lower.split_once(" of ")) {
            Some((number, total)) => (number.trim_end(), Some(total.trim_start())),
            None => (lower.as_str(), None),
        };
        let number = digits(number).ok_or(ValueError::Malformed {
            field: Field::Number,
        })?;
        let total = total
            .map(|total| {
                digits(total).ok_or(ValueError::Malformed {
                    field: Field::Total,
                })
            })
            .transpose()?;
        Self::new(number, total)
    }

    /// The number.
    #[must_use]
    pub const fn number(self) -> u16 {
        self.number
    }

    /// The total, when known.
    #[must_use]
    pub const fn total(self) -> Option<u16> {
        self.total
    }
}

/// A release or recording date that may lack its month or day, in the
/// years 1 to 9999.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PartialDate {
    year: u16,
    month: Option<u8>,
    day: Option<u8>,
}

impl PartialDate {
    /// The date `year`, `year`-`month` or `year`-`month`-`day`.
    ///
    /// # Errors
    ///
    /// [`ValueError::OutOfRange`] for a year outside 1 to 9999 or a month
    /// or day that does not exist; [`ValueError::Malformed`] for a day
    /// without a month.
    pub fn new(year: u16, month: Option<u8>, day: Option<u8>) -> Result<Self, ValueError> {
        if !(1..=9999).contains(&year) {
            return Err(ValueError::OutOfRange {
                field: Field::Year,
                value: u64::from(year),
            });
        }
        let length = match month {
            Some(month) => days_in_month(year, month).ok_or(ValueError::OutOfRange {
                field: Field::Month,
                value: u64::from(month),
            })?,
            None if day.is_some() => return Err(ValueError::Malformed { field: Field::Day }),
            None => 0,
        };
        match day {
            Some(day) if !(1..=length).contains(&day) => Err(ValueError::OutOfRange {
                field: Field::Day,
                value: u64::from(day),
            }),
            _ => Ok(Self { year, month, day }),
        }
    }

    /// Reads `YYYY`, `YYYY-MM` or `YYYY-MM-DD`, ignoring anything after a
    /// `T` or a space, such as the time in an ID3v2.4 or MP4 timestamp.
    ///
    /// # Errors
    ///
    /// [`ValueError::Malformed`] for other text, and the errors of
    /// [`Self::new`].
    pub fn parse(text: Untrusted<&str>) -> Result<Self, ValueError> {
        let text = text.into_inner();
        let text = text.trim();
        let date = text
            .split_once(['T', 't', ' '])
            .map_or(text, |(date, _)| date);
        let fixed = |part: &str, len: usize, field: Field| {
            digits(part)
                .filter(|_| part.len() == len)
                .ok_or(ValueError::Malformed { field })
        };
        let mut parts = date.split('-');
        let year = fixed(parts.next().unwrap_or_default(), 4, Field::Year)?;
        let month = parts
            .next()
            .map(|part| fixed(part, 2, Field::Month))
            .transpose()?;
        let day = parts
            .next()
            .map(|part| fixed(part, 2, Field::Day))
            .transpose()?;
        if parts.next().is_some() {
            return Err(ValueError::Malformed { field: Field::Day });
        }
        Self::new(narrow(year), month.map(narrow), day.map(narrow))
    }

    /// The year.
    #[must_use]
    pub const fn year(self) -> u16 {
        self.year
    }

    /// The month, when known.
    #[must_use]
    pub const fn month(self) -> Option<u8> {
        self.month
    }

    /// The day, when known.
    #[must_use]
    pub const fn day(self) -> Option<u8> {
        self.day
    }
}

/// Converts the value of at most four digits, which always fits.
fn narrow<T: TryFrom<u64> + Default>(value: u64) -> T {
    T::try_from(value).unwrap_or_default()
}

/// Reads an optionally signed decimal with digits on both sides of an
/// optional `.` or `,`; no exponent, no `inf`, no `nan`.
fn decimal(text: &str) -> Option<f32> {
    let (sign, unsigned) = match text.strip_prefix('-') {
        Some(unsigned) => ("-", unsigned),
        None => ("", text.strip_prefix('+').unwrap_or(text)),
    };
    let (whole, fraction) = unsigned.split_once(['.', ',']).unwrap_or((unsigned, "0"));
    let all_digits = |part: &str| digits(part).is_some();
    if all_digits(whole) && all_digits(fraction) {
        format!("{sign}{whole}.{fraction}").parse().ok()
    } else {
        None
    }
}

/// A `ReplayGain` or R128 gain in decibels, finite and from −128 to +128,
/// which holds every R128 value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GainDb(f32);

impl GainDb {
    /// The lowest gain accepted.
    pub const MIN: f32 = -128.0;
    /// The highest gain accepted.
    pub const MAX: f32 = 128.0;

    /// A gain of `db` decibels.
    ///
    /// # Errors
    ///
    /// [`ValueError::Unusable`] for a value that is not finite or is
    /// outside [`Self::MIN`] to [`Self::MAX`].
    pub fn new(db: f32) -> Result<Self, ValueError> {
        if (Self::MIN..=Self::MAX).contains(&db) {
            Ok(Self(db))
        } else {
            Err(ValueError::Unusable { field: Field::Gain })
        }
    }

    /// Reads a `ReplayGain` value such as `-6.5 dB`, `+3.0 dB` or `-6,5 dB`;
    /// the unit may be missing or in any case.
    ///
    /// # Errors
    ///
    /// [`ValueError::Malformed`] for other text, and the errors of
    /// [`Self::new`].
    pub fn parse(text: Untrusted<&str>) -> Result<Self, ValueError> {
        let text = text.into_inner();
        let lower = text.trim().to_ascii_lowercase();
        let number = lower.strip_suffix("db").unwrap_or(&lower).trim_end();
        Self::new(decimal(number).ok_or(ValueError::Malformed { field: Field::Gain })?)
    }

    /// The gain of an R128 tag or an Opus header: a signed Q7.8 fixed-point
    /// number of decibels. Every such value is in range.
    #[must_use]
    pub fn from_q7_8(raw: i16) -> Self {
        Self(f32::from(raw) / 256.0)
    }

    /// The gain in decibels.
    #[must_use]
    pub const fn db(self) -> f32 {
        self.0
    }
}

/// A `ReplayGain` peak: the largest sample as a ratio of full scale, finite
/// and from 0 to 16.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeakRatio(f32);

impl PeakRatio {
    /// The highest peak accepted, 24 dB over full scale.
    pub const MAX: f32 = 16.0;

    /// A peak of `ratio` times full scale.
    ///
    /// # Errors
    ///
    /// [`ValueError::Unusable`] for a value that is not finite or is
    /// outside 0 to [`Self::MAX`].
    pub fn new(ratio: f32) -> Result<Self, ValueError> {
        if (0.0..=Self::MAX).contains(&ratio) {
            Ok(Self(ratio))
        } else {
            Err(ValueError::Unusable { field: Field::Peak })
        }
    }

    /// Reads a peak such as `0.988525` or `0,988525`.
    ///
    /// # Errors
    ///
    /// [`ValueError::Malformed`] for other text, and the errors of
    /// [`Self::new`].
    pub fn parse(text: Untrusted<&str>) -> Result<Self, ValueError> {
        let text = text.into_inner();
        Self::new(decimal(text.trim()).ok_or(ValueError::Malformed { field: Field::Peak })?)
    }

    /// The peak as a ratio of full scale.
    #[must_use]
    pub const fn ratio(self) -> f32 {
        self.0
    }
}

/// How long a recording plays, in milliseconds, at most 30 days.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duration(u64);

impl Duration {
    /// The longest duration accepted: 30 days. A longer claim comes from a
    /// damaged header.
    pub const MAX_MILLIS: u64 = 2_592_000_000;

    /// A duration of `millis` milliseconds.
    ///
    /// # Errors
    ///
    /// [`ValueError::OutOfRange`] above [`Self::MAX_MILLIS`].
    pub fn from_millis(millis: u64) -> Result<Self, ValueError> {
        if millis <= Self::MAX_MILLIS {
            Ok(Self(millis))
        } else {
            Err(ValueError::OutOfRange {
                field: Field::Duration,
                value: millis,
            })
        }
    }

    /// The duration of `ticks` at `per_second` ticks a second, such as
    /// samples at a sample rate or units of an MP4 timescale, rounded down
    /// to the millisecond. The divisor cannot be zero.
    ///
    /// # Errors
    ///
    /// [`ValueError::OutOfRange`] above [`Self::MAX_MILLIS`].
    pub fn from_ticks(ticks: u64, per_second: NonZeroU32) -> Result<Self, ValueError> {
        let millis = u128::from(ticks).saturating_mul(1_000) / NonZeroU128::from(per_second);
        Self::from_millis(u64::try_from(millis).unwrap_or(u64::MAX))
    }

    /// The duration in milliseconds.
    #[must_use]
    pub const fn millis(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn malformed(field: Field) -> ValueError {
        ValueError::Malformed { field }
    }

    fn out_of_range(field: Field, value: u64) -> ValueError {
        ValueError::OutOfRange { field, value }
    }

    fn unusable(field: Field) -> ValueError {
        ValueError::Unusable { field }
    }

    fn hz(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
    }

    /// The example UUID of RFC 4122 and RFC 9562.
    const EXAMPLE_BYTES: [u8; 16] = [
        0xF8, 0x1D, 0x4F, 0xAE, 0x7D, 0xEC, 0x11, 0xD0, 0xA7, 0x65, 0x00, 0xA0, 0xC9, 0x1E, 0x6B,
        0xF6,
    ];

    /// Verifies: SEC-MED-014
    #[test]
    fn reads_an_mbid_in_either_case() {
        for text in [
            "f81d4fae-7dec-11d0-a765-00a0c91e6bf6",
            "F81D4FAE-7DEC-11D0-A765-00A0C91E6BF6",
            "f81D4fAe-7dEc-11d0-A765-00a0c91E6bf6",
        ] {
            assert_eq!(
                Mbid::parse(Untrusted::new(text)),
                Ok(Mbid(EXAMPLE_BYTES)),
                "{text}"
            );
        }
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn refuses_anything_but_a_hyphenated_mbid() {
        for text in [
            "",
            "../../x",
            "{f81d4fae-7dec-11d0-a765-00a0c91e6bf6}",
            "f81d4fae7dec11d0a76500a0c91e6bf6",
            "f81d4fae-7dec-11d0-a765-00a0c91e6bf",
            "f81d4fae-7dec-11d0-a765-00a0c91e6bf6a",
            "f81d4fae-7dec-11d0-a765-00a0c91e6bf6\n",
            " f81d4fae-7dec-11d0-a765-00a0c91e6bf6",
            "f81d4fae-7dec-11d0-a765-00a0c91e6bfg",
            "f81d4fa-e7dec-11d0-a765-00a0c91e6bf6",
            "f81d4fae-7dec-11d0-a765a00a0c91e6bf6",
            "+81d4fae-7dec-11d0-a765-00a0c91e6bf6",
            "f81d4fae-7dec-11d0-a765-00a0c91e6bé",
            "../../../../../../../../../../../../",
        ] {
            assert_eq!(
                Mbid::parse(Untrusted::new(text)),
                Err(malformed(Field::Mbid)),
                "{text:?}"
            );
        }
    }

    #[test]
    fn writes_an_mbid_in_lower_case_with_hyphens() {
        assert_eq!(
            Mbid(EXAMPLE_BYTES).to_string(),
            "f81d4fae-7dec-11d0-a765-00a0c91e6bf6"
        );
        assert_eq!(
            Mbid([0; 16]).to_string(),
            "00000000-0000-0000-0000-000000000000"
        );
        assert_eq!(
            Mbid([0xFF; 16]).to_string(),
            "ffffffff-ffff-ffff-ffff-ffffffffffff"
        );
        assert_eq!(
            Mbid([
                0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x10, 0x32, 0x54, 0x76, 0x98, 0xBA,
                0xDC, 0xFE
            ])
            .to_string(),
            "01234567-89ab-cdef-1032-547698badcfe"
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn reads_an_isrc_with_or_without_hyphens_in_either_case() {
        for text in ["USS1Z9900001", "uss1z9900001", "US-S1Z-99-00001"] {
            let isrc = Isrc::parse(Untrusted::new(text)).unwrap();
            assert_eq!(isrc.as_str(), "USS1Z9900001", "{text}");
        }
        assert_eq!(
            Isrc::parse(Untrusted::new("GBAYE0601498"))
                .unwrap()
                .as_str(),
            "GBAYE0601498"
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn refuses_anything_but_an_isrc() {
        for text in [
            "",
            "USS1Z990000",
            "USS1Z99000012",
            "US-S1Z-9900001",
            "US_S1Z_99_00001",
            "1SS1Z9900001",
            "U1S1Z9900001",
            "USS!Z9900001",
            "USS1Z9A00001",
            "USS1Z990000A",
            "USS1Z99 0001",
            "ÜSS1Z9900001",
            "../../../../x",
        ] {
            assert_eq!(
                Isrc::parse(Untrusted::new(text)),
                Err(malformed(Field::Isrc)),
                "{text:?}"
            );
        }
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn holds_sample_rates_channels_and_depths_to_their_ranges() {
        assert_eq!(SampleRate::new(1).map(SampleRate::hz), Ok(hz(1)));
        assert_eq!(SampleRate::new(44_100).map(SampleRate::hz), Ok(hz(44_100)));
        assert_eq!(
            SampleRate::new(768_000).map(SampleRate::hz),
            Ok(hz(768_000))
        );
        assert_eq!(Channels::new(1).map(Channels::get), Ok(hz(1)));
        assert_eq!(Channels::new(255).map(Channels::get), Ok(hz(255)));
        assert_eq!(BitDepth::new(1).map(BitDepth::get), Ok(hz(1)));
        assert_eq!(BitDepth::new(64).map(BitDepth::get), Ok(hz(64)));
        for value in [0, 768_001, u32::MAX] {
            assert_eq!(
                SampleRate::new(value),
                Err(out_of_range(Field::SampleRate, u64::from(value)))
            );
        }
        for value in [0, 256, u32::MAX] {
            assert_eq!(
                Channels::new(value),
                Err(out_of_range(Field::Channels, u64::from(value)))
            );
        }
        for value in [0, 65, u32::MAX] {
            assert_eq!(
                BitDepth::new(value),
                Err(out_of_range(Field::BitDepth, u64::from(value)))
            );
        }
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn reads_track_and_disc_numbers_in_every_common_form() {
        let cases = [
            ("3", 3, None),
            ("3/12", 3, Some(12)),
            ("3 / 12", 3, Some(12)),
            ("03 of 12", 3, Some(12)),
            ("3 OF 12", 3, Some(12)),
            (" 7 ", 7, None),
            ("12/12", 12, Some(12)),
            ("1", 1, None),
            ("9999/9999", 9999, Some(9999)),
        ];
        for (text, number, total) in cases {
            let parsed = NumberOf::parse(Untrusted::new(text));
            assert_eq!(parsed, Ok(NumberOf { number, total }), "{text}");
            assert_eq!(parsed.map(NumberOf::number), Ok(number));
            assert_eq!(parsed.map(NumberOf::total), Ok(total));
        }
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn refuses_track_numbers_that_are_malformed_or_out_of_range() {
        let cases = [
            ("", malformed(Field::Number)),
            ("/12", malformed(Field::Number)),
            ("three", malformed(Field::Number)),
            ("+3", malformed(Field::Number)),
            ("-3", malformed(Field::Number)),
            ("3.0", malformed(Field::Number)),
            ("3 of", malformed(Field::Number)),
            ("of 12", malformed(Field::Number)),
            ("3of12", malformed(Field::Number)),
            ("\u{663}", malformed(Field::Number)),
            ("3/", malformed(Field::Total)),
            ("3/12/1", malformed(Field::Total)),
            ("3/x", malformed(Field::Total)),
            ("0", out_of_range(Field::Number, 0)),
            ("10000", out_of_range(Field::Number, 10_000)),
            (
                "99999999999999999999999",
                out_of_range(Field::Number, u64::MAX),
            ),
            ("3/0", out_of_range(Field::Total, 0)),
            ("3/10000", out_of_range(Field::Total, 10_000)),
            (
                "13/12",
                ValueError::AboveTotal {
                    number: 13,
                    total: 12,
                },
            ),
        ];
        for (text, error) in cases {
            assert_eq!(
                NumberOf::parse(Untrusted::new(text)),
                Err(error),
                "{text:?}"
            );
        }
        assert_eq!(
            NumberOf::new(70_000, None),
            Err(out_of_range(Field::Number, 70_000))
        );
        assert_eq!(
            NumberOf::new(5, Some(4)),
            Err(ValueError::AboveTotal {
                number: 5,
                total: 4
            })
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn reads_partial_dates() {
        let cases = [
            ("2019", 2019, None, None),
            ("2019-05", 2019, Some(5), None),
            ("2019-05-17", 2019, Some(5), Some(17)),
            ("2019-05-17T07:00:00Z", 2019, Some(5), Some(17)),
            ("2019-05-17t07:00", 2019, Some(5), Some(17)),
            ("2019-05-17 07:00", 2019, Some(5), Some(17)),
            (" 2019 ", 2019, None, None),
            ("2020-02-29", 2020, Some(2), Some(29)),
            ("0001", 1, None, None),
            ("9999-12-31", 9999, Some(12), Some(31)),
        ];
        for (text, year, month, day) in cases {
            let parsed = PartialDate::parse(Untrusted::new(text));
            assert_eq!(parsed, Ok(PartialDate { year, month, day }), "{text}");
            assert_eq!(parsed.map(PartialDate::year), Ok(year));
            assert_eq!(parsed.map(PartialDate::month), Ok(month));
            assert_eq!(parsed.map(PartialDate::day), Ok(day));
        }
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn refuses_dates_that_are_malformed_or_do_not_exist() {
        let cases = [
            ("", malformed(Field::Year)),
            ("19", malformed(Field::Year)),
            ("20190", malformed(Field::Year)),
            ("May 2019", malformed(Field::Year)),
            ("2019/05/17", malformed(Field::Year)),
            ("\u{FF12}\u{FF10}\u{FF11}\u{FF19}", malformed(Field::Year)),
            ("2019-", malformed(Field::Month)),
            ("2019-5", malformed(Field::Month)),
            ("2019-05-", malformed(Field::Day)),
            ("2019-05-7", malformed(Field::Day)),
            ("2019-05-17-01", malformed(Field::Day)),
            ("0000", out_of_range(Field::Year, 0)),
            ("2019-00", out_of_range(Field::Month, 0)),
            ("2019-13", out_of_range(Field::Month, 13)),
            ("2019-01-00", out_of_range(Field::Day, 0)),
            ("2019-02-29", out_of_range(Field::Day, 29)),
            ("2019-04-31", out_of_range(Field::Day, 31)),
        ];
        for (text, error) in cases {
            assert_eq!(
                PartialDate::parse(Untrusted::new(text)),
                Err(error),
                "{text:?}"
            );
        }
        assert_eq!(
            PartialDate::new(2019, None, Some(5)),
            Err(malformed(Field::Day))
        );
        assert_eq!(
            PartialDate::new(10_000, None, None),
            Err(out_of_range(Field::Year, 10_000))
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn reads_replaygain_values() {
        let cases = [
            ("-6.5 dB", -6.5),
            ("+3.0 dB", 3.0),
            ("-6,5 dB", -6.5),
            ("-6.50 DB", -6.5),
            ("-6.5dB", -6.5),
            ("  -6.5 dB  ", -6.5),
            ("-6.5", -6.5),
            ("0 dB", 0.0),
            ("128 dB", 128.0),
            ("-128.00 dB", -128.0),
        ];
        for (text, db) in cases {
            let parsed = GainDb::parse(Untrusted::new(text));
            assert_eq!(parsed, Ok(GainDb(db)), "{text}");
            assert_eq!(parsed.map(GainDb::db), Ok(db));
        }
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn refuses_replaygain_values_that_are_malformed_or_unusable() {
        for text in [
            "",
            "dB",
            "nan",
            "NaN dB",
            "inf",
            "-inf dB",
            "infinity",
            "+1e308 dB",
            "1e3",
            "- 6.5 dB",
            "6.",
            ".5",
            "6.5.1",
            "6,5,1",
            "--6",
            "+-6",
            "6 5",
            "0x10",
            "\u{FF16}",
            "6.5 dB dB",
        ] {
            assert_eq!(
                GainDb::parse(Untrusted::new(text)),
                Err(malformed(Field::Gain)),
                "{text:?}"
            );
        }
        let huge = format!("1{} dB", "0".repeat(50));
        for text in ["128.01 dB", "-128.5", "1000 dB", huge.as_str()] {
            assert_eq!(
                GainDb::parse(Untrusted::new(text)),
                Err(unusable(Field::Gain)),
                "{text:?}"
            );
        }
        // The nearest floats beyond each end of the range.
        let above = f32::from_bits(128.0_f32.to_bits() + 1);
        let below = f32::from_bits((-128.0_f32).to_bits() + 1);
        for db in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, above, below] {
            assert_eq!(GainDb::new(db), Err(unusable(Field::Gain)), "{db}");
        }
        assert_eq!(GainDb::new(-128.0), Ok(GainDb(-128.0)));
        assert_eq!(GainDb::new(128.0), Ok(GainDb(128.0)));
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn reads_r128_gains_as_q7_8_fixed_point() {
        let cases = [
            (0, 0.0),
            (256, 1.0),
            (-256, -1.0),
            (-1664, -6.5),
            (1, 0.003_906_25),
            (i16::MIN, -128.0),
            // 32767 / 256 = 127.99609375, exact in binary.
            (i16::MAX, f32::from_bits(0x42FF_FE00)),
        ];
        for (raw, db) in cases {
            assert_eq!(GainDb::from_q7_8(raw), GainDb(db), "{raw}");
        }
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn reads_peaks_and_refuses_unusable_ones() {
        let cases = [
            ("0.988525", 0.988_525),
            ("1.000000", 1.0),
            ("1,5", 1.5),
            (" 0 ", 0.0),
            ("16", 16.0),
        ];
        for (text, ratio) in cases {
            let parsed = PeakRatio::parse(Untrusted::new(text));
            assert_eq!(parsed, Ok(PeakRatio(ratio)), "{text}");
            assert_eq!(parsed.map(PeakRatio::ratio), Ok(ratio));
        }
        for text in ["", "nan", "inf", "1e-3", "0.5 dB", "-", "0..5"] {
            assert_eq!(
                PeakRatio::parse(Untrusted::new(text)),
                Err(malformed(Field::Peak)),
                "{text:?}"
            );
        }
        for text in ["16.01", "-0.1", "1000"] {
            assert_eq!(
                PeakRatio::parse(Untrusted::new(text)),
                Err(unusable(Field::Peak)),
                "{text:?}"
            );
        }
        let above = f32::from_bits(16.0_f32.to_bits() + 1);
        for ratio in [f32::NAN, f32::INFINITY, -f32::MIN_POSITIVE, above] {
            assert_eq!(PeakRatio::new(ratio), Err(unusable(Field::Peak)), "{ratio}");
        }
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn computes_durations_without_dividing_by_zero() {
        assert_eq!(Duration::from_millis(0).map(Duration::millis), Ok(0));
        assert_eq!(
            Duration::from_millis(2_592_000_000).map(Duration::millis),
            Ok(2_592_000_000)
        );
        assert_eq!(
            Duration::from_millis(2_592_000_001),
            Err(out_of_range(Field::Duration, 2_592_000_001))
        );
        let rate = SampleRate::new(44_100).unwrap().hz();
        let cases = [
            (44_100, 1_000),
            (1, 0),
            (44_099, 999),
            (88_200, 2_000),
            (0, 0),
        ];
        for (samples, millis) in cases {
            assert_eq!(
                Duration::from_ticks(samples, rate),
                Ok(Duration(millis)),
                "{samples}"
            );
        }
        assert_eq!(
            Duration::from_ticks(2_592_000, hz(1)),
            Ok(Duration(2_592_000_000))
        );
        assert_eq!(
            Duration::from_ticks(2_592_001, hz(1)),
            Err(out_of_range(Field::Duration, 2_592_001_000))
        );
        assert_eq!(
            Duration::from_ticks(u64::MAX, hz(1)),
            Err(out_of_range(Field::Duration, u64::MAX))
        );
        // 2^64 - 1 is (2^32 - 1)(2^32 + 1), so this divides exactly.
        assert_eq!(
            Duration::from_ticks(u64::MAX, hz(u32::MAX)),
            Err(out_of_range(Field::Duration, 4_294_967_297_000))
        );
    }

    proptest! {
        /// Verifies: SEC-MED-014
        #[test]
        fn writing_then_reading_an_mbid_returns_it(bytes in any::<[u8; 16]>()) {
            prop_assert_eq!(Mbid::parse(Untrusted::new(&Mbid(bytes).to_string())), Ok(Mbid(bytes)));
        }

        /// Verifies: SEC-MED-014
        #[test]
        fn writing_then_reading_a_number_returns_it(
            number in 1_u16..=9999,
            extra in 0_u16..=9999,
            form in 0_usize..3,
        ) {
            let total = number.saturating_add(extra).min(9999);
            let text = match form {
                0 => format!("{number}/{total}"),
                1 => format!("{number:04} of {total}"),
                _ => format!("{number}"),
            };
            let expected = NumberOf { number, total: (form < 2).then_some(total) };
            prop_assert_eq!(NumberOf::parse(Untrusted::new(&text)), Ok(expected));
        }

        /// Whatever any parser accepts lies inside its documented range.
        ///
        /// Verifies: SEC-MED-014
        #[test]
        fn accepted_values_lie_inside_their_ranges(
            text in prop_oneof![
                // Always valid, so every check below runs on every test run.
                "[1-9][0-9]{0,2} ?/ ?[1-9][0-9]{3}",
                "[1-9][0-9]{3}-(0[1-9]|1[0-2])-(0[1-9]|1[0-9]|2[0-8])(T[0-9:]{0,8})?",
                "[-+]?[0-9]{1,3}([.,][0-9]{1,3})?( ?[dD][bB])?",
                "[0-9]([.,][0-9]{1,6})?",
                "[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}",
                // Near misses and noise.
                "[0-9]{1,5}( ?/ ?[0-9]{1,5}| [oO][fF] [0-9]{1,5})?",
                "[0-9]{4}(-[0-9]{1,2}(-[0-9]{1,2})?)?([Tt ].{0,8})?",
                "[0-9a-fA-F-]{36}",
                ".{0,40}",
            ],
        ) {
            if let Ok(number) = NumberOf::parse(Untrusted::new(&text)) {
                prop_assert!((1..=9999).contains(&number.number()));
                prop_assert!(number.total().is_none_or(|total| (number.number()..=9999).contains(&total)));
            }
            if let Ok(date) = PartialDate::parse(Untrusted::new(&text)) {
                prop_assert!((1..=9999).contains(&date.year()));
                prop_assert!(date.month().is_none_or(|month| (1..=12).contains(&month)));
                prop_assert!(date.day().is_none_or(|day| date.month().is_some() && (1..=31).contains(&day)));
            }
            if let Ok(gain) = GainDb::parse(Untrusted::new(&text)) {
                prop_assert!(gain.db().is_finite() && (-128.0..=128.0).contains(&gain.db()));
            }
            if let Ok(peak) = PeakRatio::parse(Untrusted::new(&text)) {
                prop_assert!(peak.ratio().is_finite() && (0.0..=16.0).contains(&peak.ratio()));
            }
            if let Ok(mbid) = Mbid::parse(Untrusted::new(&text)) {
                prop_assert_eq!(mbid.to_string(), text.to_ascii_lowercase());
            }
        }

        /// Verifies: SEC-MED-014
        #[test]
        fn every_q7_8_gain_is_exact_and_in_range(raw in any::<i16>()) {
            let gain = GainDb::from_q7_8(raw);
            prop_assert_eq!(GainDb::new(gain.db()), Ok(gain));
            prop_assert_eq!((gain.db() * 256.0).to_bits(), f32::from(raw).to_bits());
        }
    }
}
