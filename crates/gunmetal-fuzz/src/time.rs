//! The harness for the RFC 3339 reader and writer in `gunmetal_core::time`.

use gunmetal_core::time::{self, CivilDate, TimeError, Timestamp};
use gunmetal_core::untrusted::Untrusted;

/// What the RFC 3339 reader reported for one input, read as UTF-8 with
/// each invalid sequence replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    /// [`time::parse_rfc3339`]: milliseconds since the Unix epoch, or why
    /// the text is not a time.
    pub millis: Result<i64, TimeError>,
}

/// Milliseconds in a day.
const MS_PER_DAY: i64 = 86_400_000;

/// Feeds `data` to [`time::parse_rfc3339`], writes what it accepts with
/// [`time::format_rfc3339`], and takes the day it falls on through
/// [`CivilDate`].
///
/// # Panics
///
/// Panics when an accepted time is outside [`Timestamp::MIN`] to
/// [`Timestamp::MAX`] or is not the timestamp of its own milliseconds;
/// when it is not written as `YYYY-MM-DDTHH:MM:SS.mmmZ` or does not read
/// back as the same time; or when the day it falls on is not a date, does
/// not count back to the same day number, cannot be built again from its
/// year, month and day, has a day past the end of its month, or is not the
/// date the written form starts with.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let text = String::from_utf8_lossy(data);
    let parsed = time::parse_rfc3339(Untrusted::new(&text));
    if let Ok(instant) = parsed {
        let written = time::format_rfc3339(instant);
        let days = instant.millis().div_euclid(MS_PER_DAY);
        assert!(
            (Timestamp::MIN..=Timestamp::MAX).contains(&instant)
                && Timestamp::from_millis(instant.millis()) == Ok(instant)
                && written.len() == 24
                && written.ends_with('Z')
                && time::parse_rfc3339(Untrusted::new(&written)) == Ok(instant)
                && CivilDate::from_days(days).is_ok_and(|date| {
                    date.days() == days
                        && CivilDate::new(date.year(), date.month(), date.day()) == Ok(date)
                        && time::days_in_month(date.year(), date.month())
                            .is_some_and(|length| (1..=length).contains(&date.day()))
                        && written.starts_with(&format!(
                            "{:04}-{:02}-{:02}T",
                            date.year(),
                            date.month(),
                            date.day()
                        ))
                }),
            "{text:?} read as {instant:?}, written {written:?}"
        );
    }
    Outcome {
        millis: parsed.map(Timestamp::millis),
    }
}
