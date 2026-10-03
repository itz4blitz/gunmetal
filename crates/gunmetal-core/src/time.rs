//! Time: millisecond timestamps, civil dates and RFC 3339.
//!
//! The core never reads a clock. It takes `now` as an argument, and the
//! server reads the time through a [`Clock`] that tests replace. Time zones
//! are the device's business; everything here is UTC.
//!
//! The date arithmetic follows Howard Hinnant's `days_from_civil` and
//! `civil_from_days`, which count years from March so that the leap day
//! falls at the end of a year. Every input is first held to the years 0000
//! to 9999, so no step can overflow; the saturating operations only satisfy
//! the lint that forbids unchecked arithmetic.

/// Milliseconds in a day.
const MS_PER_DAY: i64 = 86_400_000;
/// The day number of 0000-01-01.
const FIRST_DAY: i64 = -719_528;
/// The day number of 9999-12-31.
const LAST_DAY: i64 = 2_932_896;

/// Milliseconds since 1970-01-01T00:00:00Z, from 0000-01-01 to the end of
/// 9999-12-31, the years RFC 3339 can write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(i64);

impl Timestamp {
    /// 0000-01-01T00:00:00.000Z.
    pub const MIN: Self = Self(-62_167_219_200_000);
    /// 9999-12-31T23:59:59.999Z.
    pub const MAX: Self = Self(253_402_300_799_999);

    /// The timestamp `millis` milliseconds after the Unix epoch.
    ///
    /// # Errors
    ///
    /// [`TimeError::OutOfRange`] outside [`Self::MIN`] to [`Self::MAX`].
    pub fn from_millis(millis: i64) -> Result<Self, TimeError> {
        if (Self::MIN.0..=Self::MAX.0).contains(&millis) {
            Ok(Self(millis))
        } else {
            Err(TimeError::OutOfRange { millis })
        }
    }

    /// Milliseconds since the Unix epoch.
    #[must_use]
    pub const fn millis(self) -> i64 {
        self.0
    }
}

/// Where the server reads the time from; tests supply their own.
pub trait Clock {
    /// The current time.
    fn now(&self) -> Timestamp;
}

/// A day in the proleptic Gregorian calendar, from 0000-01-01 to
/// 9999-12-31.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CivilDate {
    year: u16,
    month: u8,
    day: u8,
}

impl CivilDate {
    /// The date `year`-`month`-`day`.
    ///
    /// # Errors
    ///
    /// [`TimeError::InvalidDate`] when that day does not exist or the year
    /// is after 9999.
    pub fn new(year: u16, month: u8, day: u8) -> Result<Self, TimeError> {
        let exists = year <= 9999
            && days_in_month(year, month).is_some_and(|length| (1..=length).contains(&day));
        if exists {
            Ok(Self { year, month, day })
        } else {
            Err(TimeError::InvalidDate { year, month, day })
        }
    }

    /// The date `days` days after 1970-01-01.
    ///
    /// # Errors
    ///
    /// [`TimeError::DayOutOfRange`] before 0000-01-01 or after 9999-12-31.
    pub fn from_days(days: i64) -> Result<Self, TimeError> {
        if (FIRST_DAY..=LAST_DAY).contains(&days) {
            let (year, month, day) = civil(days);
            Ok(Self { year, month, day })
        } else {
            Err(TimeError::DayOutOfRange { days })
        }
    }

    /// Days after 1970-01-01, negative before it.
    #[must_use]
    pub fn days(self) -> i64 {
        let month = i64::from(self.month);
        let year = i64::from(self.year).saturating_sub(i64::from(self.month <= 2));
        let era = year.div_euclid(400);
        let year_of_era = year.rem_euclid(400);
        let month_from_march = month.saturating_add(9) % 12;
        let day_of_year = (month_from_march.saturating_mul(153).saturating_add(2) / 5)
            .saturating_add(i64::from(self.day))
            .saturating_sub(1);
        let day_of_era = year_of_era
            .saturating_mul(365)
            .saturating_add(year_of_era / 4)
            .saturating_sub(year_of_era / 100)
            .saturating_add(day_of_year);
        era.saturating_mul(146_097)
            .saturating_add(day_of_era)
            .saturating_sub(719_468)
    }

    /// The year, 0 to 9999.
    #[must_use]
    pub const fn year(self) -> u16 {
        self.year
    }

    /// The month, 1 to 12.
    #[must_use]
    pub const fn month(self) -> u8 {
        self.month
    }

    /// The day of the month, 1 to 31.
    #[must_use]
    pub const fn day(self) -> u8 {
        self.day
    }
}

/// The year, month and day of a day number from [`FIRST_DAY`] to
/// [`LAST_DAY`].
fn civil(days: i64) -> (u16, u8, u8) {
    let from_march_of_year_0 = days.saturating_add(719_468);
    let era = from_march_of_year_0.div_euclid(146_097);
    let day_of_era = from_march_of_year_0.rem_euclid(146_097);
    let year_of_era = day_of_era
        .saturating_sub(day_of_era / 1_460)
        .saturating_add(day_of_era / 36_524)
        .saturating_sub(day_of_era / 146_096)
        / 365;
    let day_of_year = day_of_era.saturating_sub(
        year_of_era
            .saturating_mul(365)
            .saturating_add(year_of_era / 4)
            .saturating_sub(year_of_era / 100),
    );
    let month_from_march = day_of_year.saturating_mul(5).saturating_add(2) / 153;
    let day = day_of_year
        .saturating_sub(month_from_march.saturating_mul(153).saturating_add(2) / 5)
        .saturating_add(1);
    let month = (month_from_march.saturating_add(2) % 12).saturating_add(1);
    let year = era
        .saturating_mul(400)
        .saturating_add(year_of_era)
        .saturating_add(i64::from(month <= 2));
    (narrow(year), narrow(month), narrow(day))
}

/// Converts a value already known to fit.
fn narrow<T: TryFrom<i64> + Default>(value: i64) -> T {
    T::try_from(value).unwrap_or_default()
}

/// How many days `month` has in `year`, or `None` when `month` is not 1 to
/// 12.
#[must_use]
pub fn days_in_month(year: u16, month: u8) -> Option<u8> {
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 if leap => Some(29),
        2 => Some(28),
        _ => None,
    }
}

/// Why a time could not be built or parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeError {
    /// A timestamp outside the years 0000 to 9999.
    OutOfRange {
        /// Milliseconds since the Unix epoch.
        millis: i64,
    },
    /// A day number outside the years 0000 to 9999.
    DayOutOfRange {
        /// Days since 1970-01-01.
        days: i64,
    },
    /// A day that does not exist, such as 2023-02-29.
    InvalidDate {
        /// The year given.
        year: u16,
        /// The month given.
        month: u8,
        /// The day given.
        day: u8,
    },
    /// A time of day that does not exist. Leap seconds are refused.
    InvalidTime {
        /// The hour given.
        hour: u8,
        /// The minute given.
        minute: u8,
        /// The second given.
        second: u8,
    },
    /// A UTC offset beyond 23:59.
    InvalidOffset {
        /// The hours given.
        hours: u8,
        /// The minutes given.
        minutes: u8,
    },
    /// Text that is not an RFC 3339 date-time.
    Syntax,
}

/// Writes `time` as `YYYY-MM-DDTHH:MM:SS.mmmZ` (RFC 3339), always with
/// milliseconds and always in UTC.
#[must_use]
pub fn format_rfc3339(time: Timestamp) -> String {
    let (year, month, day) = civil(time.0.div_euclid(MS_PER_DAY));
    let millis = time.0.rem_euclid(MS_PER_DAY);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        millis / 3_600_000,
        millis / 60_000 % 60,
        millis / 1_000 % 60,
        millis % 1_000,
    )
}

/// Reads an RFC 3339 date-time, such as `2009-02-13T23:31:30.123+01:00`.
///
/// The separators `T` and `Z` may be lower case. A fraction of up to nine
/// digits is kept to the millisecond, rounding toward the past.
///
/// # Errors
///
/// [`TimeError::Syntax`] for text outside RFC 3339's grammar; the other
/// variants for a date, time or offset that does not exist (leap seconds
/// included), or an instant outside the years 0000 to 9999.
pub fn parse_rfc3339(text: &str) -> Result<Timestamp, TimeError> {
    let (head, rest) = text
        .as_bytes()
        .split_at_checked(19)
        .ok_or(TimeError::Syntax)?;
    let &[
        y1,
        y2,
        y3,
        y4,
        b'-',
        mo1,
        mo2,
        b'-',
        d1,
        d2,
        b'T' | b't',
        h1,
        h2,
        b':',
        mi1,
        mi2,
        b':',
        s1,
        s2,
    ] = head
    else {
        return Err(TimeError::Syntax);
    };
    let fields = [
        &[y1, y2, y3, y4][..],
        &[mo1, mo2],
        &[d1, d2],
        &[h1, h2],
        &[mi1, mi2],
        &[s1, s2],
    ]
    .map(decimal);
    let [
        Some(year),
        Some(month),
        Some(day),
        Some(hour),
        Some(minute),
        Some(second),
    ] = fields
    else {
        return Err(TimeError::Syntax);
    };
    let (hour, minute, second) = (narrow_u32(hour), narrow_u32(minute), narrow_u32(second));

    let (fraction, zone) = match rest {
        [b'.', tail @ ..] => {
            let digits = tail.iter().take_while(|byte| byte.is_ascii_digit()).count();
            if !(1..=9).contains(&digits) {
                return Err(TimeError::Syntax);
            }
            tail.split_at_checked(digits).unwrap_or_default()
        }
        _ => (&[][..], rest),
    };
    let offset_minutes = match *zone {
        [b'Z' | b'z'] => 0,
        [sign @ (b'+' | b'-'), oh1, oh2, b':', om1, om2] => {
            let [Some(hours), Some(minutes)] = [&[oh1, oh2], &[om1, om2]].map(decimal) else {
                return Err(TimeError::Syntax);
            };
            let (hours, minutes): (u8, u8) = (narrow_u32(hours), narrow_u32(minutes));
            if hours > 23 || minutes > 59 {
                return Err(TimeError::InvalidOffset { hours, minutes });
            }
            let magnitude = i64::from(hours)
                .saturating_mul(60)
                .saturating_add(i64::from(minutes));
            if sign == b'-' {
                magnitude.saturating_neg()
            } else {
                magnitude
            }
        }
        _ => return Err(TimeError::Syntax),
    };

    let date = CivilDate::new(narrow_u32(year), narrow_u32(month), narrow_u32(day))?;
    if hour > 23 || minute > 59 || second > 59 {
        return Err(TimeError::InvalidTime {
            hour,
            minute,
            second,
        });
    }
    let millis_of_second = decimal(fraction.iter().chain(b"000").take(3)).unwrap_or_default();
    let millis = date
        .days()
        .saturating_mul(MS_PER_DAY)
        .saturating_add(i64::from(hour).saturating_mul(3_600_000))
        .saturating_add(i64::from(minute).saturating_mul(60_000))
        .saturating_add(i64::from(second).saturating_mul(1_000))
        .saturating_add(i64::from(millis_of_second))
        .saturating_sub(offset_minutes.saturating_mul(60_000));
    Timestamp::from_millis(millis)
}

/// The value of a run of ASCII decimal digits, or `None` if any octet is
/// not one.
fn decimal<'a>(digits: impl IntoIterator<Item = &'a u8>) -> Option<u32> {
    digits.into_iter().try_fold(0_u32, |value, &digit| {
        char::from(digit)
            .to_digit(10)
            .map(|digit| value.saturating_mul(10).saturating_add(digit))
    })
}

/// Converts the value of at most four digits, which always fits.
fn narrow_u32<T: TryFrom<u32> + Default>(value: u32) -> T {
    T::try_from(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn date(year: u16, month: u8, day: u8) -> CivilDate {
        CivilDate { year, month, day }
    }

    #[test]
    fn accepts_timestamps_from_year_0_to_year_9999() {
        for millis in [-62_167_219_200_000, -1, 0, 1, 253_402_300_799_999] {
            assert_eq!(Timestamp::from_millis(millis), Ok(Timestamp(millis)));
            assert_eq!(Timestamp(millis).millis(), millis);
        }
        for millis in [-62_167_219_200_001, 253_402_300_800_000, i64::MIN, i64::MAX] {
            assert_eq!(
                Timestamp::from_millis(millis),
                Err(TimeError::OutOfRange { millis })
            );
        }
    }

    #[test]
    fn counts_days_in_each_month_with_the_gregorian_leap_rule() {
        let long = [1, 3, 5, 7, 8, 10, 12];
        let short = [4, 6, 9, 11];
        for month in long {
            assert_eq!(days_in_month(2023, month), Some(31), "month {month}");
        }
        for month in short {
            assert_eq!(days_in_month(2023, month), Some(30), "month {month}");
        }
        // Leap years: every fourth, except centuries not divisible by 400.
        for (year, february) in [
            (0, 29),
            (4, 29),
            (1900, 28),
            (2000, 29),
            (2023, 28),
            (2024, 29),
            (2100, 28),
            (2400, 29),
            (9999, 28),
        ] {
            assert_eq!(days_in_month(year, 2), Some(february), "year {year}");
        }
        for month in [0, 13, 255] {
            assert_eq!(days_in_month(2024, month), None, "month {month}");
        }
    }

    #[test]
    fn numbers_known_days() {
        let cases = [
            (date(1970, 1, 1), 0),
            (date(1969, 12, 31), -1),
            (date(2000, 2, 29), 11_016),
            (date(2000, 3, 1), 11_017),
            (date(2100, 2, 28), 47_540),
            (date(2100, 3, 1), 47_541),
            (date(0, 1, 1), -719_528),
            (date(0, 2, 29), -719_469),
            (date(9999, 12, 31), 2_932_896),
        ];
        for (civil, days) in cases {
            assert_eq!(civil.days(), days, "{civil:?}");
            assert_eq!(CivilDate::from_days(days), Ok(civil), "day {days}");
            assert_eq!(
                CivilDate::new(civil.year(), civil.month(), civil.day()),
                Ok(civil)
            );
        }
    }

    #[test]
    fn refuses_days_outside_years_0_to_9999() {
        for days in [-719_529, 2_932_897, i64::MIN, i64::MAX] {
            assert_eq!(
                CivilDate::from_days(days),
                Err(TimeError::DayOutOfRange { days })
            );
        }
    }

    #[test]
    fn refuses_dates_that_do_not_exist() {
        for (year, month, day) in [
            (2023, 2, 29),
            (2100, 2, 29),
            (2024, 2, 30),
            (2024, 4, 31),
            (2024, 0, 1),
            (2024, 13, 1),
            (2024, 1, 0),
            (2024, 1, 32),
            (10_000, 1, 1),
            (u16::MAX, 1, 1),
        ] {
            assert_eq!(
                CivilDate::new(year, month, day),
                Err(TimeError::InvalidDate { year, month, day })
            );
        }
        assert_eq!(CivilDate::new(2024, 2, 29), Ok(date(2024, 2, 29)));
    }

    /// Walks every day from 0000-01-01 to 9999-12-31 with a counter written
    /// only for this test, and checks both directions of the conversion.
    #[test]
    fn agrees_with_a_day_counter_on_every_day_of_ten_thousand_years() {
        let leap = |year: u16| year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let mut days = -719_528_i64;
        for year in 0..=9999_u16 {
            for month in 1..=12_u8 {
                let length = match month {
                    2 if leap(year) => 29,
                    2 => 28,
                    4 | 6 | 9 | 11 => 30,
                    _ => 31,
                };
                for day in 1..=length {
                    let expected = date(year, month, day);
                    assert_eq!(CivilDate::from_days(days), Ok(expected));
                    assert_eq!(expected.days(), days, "{expected:?}");
                    days += 1;
                }
            }
        }
        assert_eq!(days, 2_932_897);
    }

    #[test]
    fn formats_rfc3339_with_milliseconds_in_utc() {
        let cases = [
            (0, "1970-01-01T00:00:00.000Z"),
            (-1, "1969-12-31T23:59:59.999Z"),
            (1_234_567_890_123, "2009-02-13T23:31:30.123Z"),
            (951_782_400_005, "2000-02-29T00:00:00.005Z"),
            (-62_167_219_200_000, "0000-01-01T00:00:00.000Z"),
            (253_402_300_799_999, "9999-12-31T23:59:59.999Z"),
            (86_399_999, "1970-01-01T23:59:59.999Z"),
        ];
        for (millis, text) in cases {
            assert_eq!(format_rfc3339(Timestamp(millis)), text);
            assert_eq!(parse_rfc3339(text), Ok(Timestamp(millis)), "{text}");
        }
    }

    #[test]
    fn parses_offsets_fractions_and_lower_case_letters() {
        let cases = [
            ("2009-02-14T01:01:30.123+01:30", 1_234_567_890_123),
            ("2009-02-13T22:31:30.123-01:00", 1_234_567_890_123),
            ("2009-02-13t23:31:30.123z", 1_234_567_890_123),
            ("1970-01-01T00:00:00Z", 0),
            ("1970-01-01T00:00:00-00:00", 0),
            ("1970-01-01T00:00:00.5Z", 500),
            ("1970-01-01T00:00:00.05Z", 50),
            ("1970-01-01T00:00:00.123456789Z", 123),
            ("1970-01-01T00:00:00+23:59", -86_340_000),
            ("1970-01-01T00:00:00-23:59", 86_340_000),
        ];
        for (text, millis) in cases {
            assert_eq!(parse_rfc3339(text), Ok(Timestamp(millis)), "{text}");
        }
    }

    #[test]
    fn refuses_text_outside_the_grammar() {
        for text in [
            "",
            "1970-01-01",
            "1970-01-01T00:00:00",
            "1970-01-01 00:00:00Z",
            "1970-01-01T00:00:00.Z",
            "1970-01-01T00:00:00.1234567890Z",
            "1970-01-01T00:00:00Zx",
            "1970-01-01T00:00:00+0100",
            "1970-01-01T00:00:00+01:0",
            "1970-01-01T00:00:00*01:00",
            "197a-01-01T00:00:00Z",
            "1970/01/01T00:00:00Z",
            "1970-1-01T00:00:00Z",
            "+1970-01-01T00:00:00Z",
            "1970-01-01T00:00:0aZ",
            "1970-01-01T00:00:00+0a:00",
            "1970-01-01T00:00:00.1a2Z",
            "１970-01-01T00:00:00Z",
        ] {
            assert_eq!(parse_rfc3339(text), Err(TimeError::Syntax), "{text:?}");
        }
    }

    #[test]
    fn refuses_fields_that_do_not_exist() {
        let cases = [
            (
                "2023-02-29T00:00:00Z",
                TimeError::InvalidDate {
                    year: 2023,
                    month: 2,
                    day: 29,
                },
            ),
            (
                "2024-13-01T00:00:00Z",
                TimeError::InvalidDate {
                    year: 2024,
                    month: 13,
                    day: 1,
                },
            ),
            (
                "1970-01-01T24:00:00Z",
                TimeError::InvalidTime {
                    hour: 24,
                    minute: 0,
                    second: 0,
                },
            ),
            (
                "1970-01-01T23:60:00Z",
                TimeError::InvalidTime {
                    hour: 23,
                    minute: 60,
                    second: 0,
                },
            ),
            (
                "1998-12-31T23:59:60Z",
                TimeError::InvalidTime {
                    hour: 23,
                    minute: 59,
                    second: 60,
                },
            ),
            (
                "1970-01-01T00:00:00+24:00",
                TimeError::InvalidOffset {
                    hours: 24,
                    minutes: 0,
                },
            ),
            (
                "1970-01-01T00:00:00-23:60",
                TimeError::InvalidOffset {
                    hours: 23,
                    minutes: 60,
                },
            ),
        ];
        for (text, error) in cases {
            assert_eq!(parse_rfc3339(text), Err(error), "{text}");
        }
    }

    #[test]
    fn refuses_instants_that_an_offset_moves_out_of_range() {
        assert_eq!(
            parse_rfc3339("0000-01-01T00:00:00+00:01"),
            Err(TimeError::OutOfRange {
                millis: -62_167_219_260_000
            })
        );
        assert_eq!(
            parse_rfc3339("9999-12-31T23:59:59.999-00:01"),
            Err(TimeError::OutOfRange {
                millis: 253_402_300_859_999
            })
        );
    }

    struct Fixed(Timestamp);

    impl Clock for Fixed {
        fn now(&self) -> Timestamp {
            self.0
        }
    }

    #[test]
    fn reads_the_time_through_a_clock() {
        let clock: &dyn Clock = &Fixed(Timestamp(42));
        assert_eq!(clock.now(), Timestamp(42));
    }

    proptest! {
        #[test]
        fn formatting_then_parsing_returns_the_timestamp(
            millis in -62_167_219_200_000_i64..=253_402_300_799_999,
        ) {
            prop_assert_eq!(parse_rfc3339(&format_rfc3339(Timestamp(millis))), Ok(Timestamp(millis)));
        }

        /// Whatever text parses names an instant that formats and parses
        /// back to itself. Function items, not closures, keep every test
        /// run's coverage the same.
        #[test]
        fn whatever_parses_round_trips(
            text in prop_oneof![
                "[0-9]{4}-(0[1-9]|1[0-2])-(0[1-9]|1[0-9]|2[0-8])[Tt]([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9](\\.[0-9]{1,9})?([Zz]|[+-]([01][0-9]|2[0-3]):[0-5][0-9])",
                "[0-9]{4}-[0-9]{2}-[0-9]{2}[Tt ][0-9]{2}:[0-9]{2}:[0-9]{2}(\\.[0-9]{0,10})?([Zz]|[+-][0-9]{2}:[0-9]{2})?",
                ".{0,40}",
            ],
        ) {
            let parsed = parse_rfc3339(&text).ok();
            let again = parsed.map(format_rfc3339).as_deref().map(parse_rfc3339);
            prop_assert_eq!(again, parsed.map(Ok));
        }
    }
}
