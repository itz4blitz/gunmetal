//! Where the log keeps a stream, and in which month's segment.
//!
//! This is the one place a stream of the core becomes the data root's name
//! for its directory, and the server's time the month of a segment (ADR 3,
//! section 3). A directory's name is built from the profile's internal ID
//! alone, never from anything a request carries (SEC-HIS-015).

use gunmetal_core::logframe::{self, SegmentHeader};
use gunmetal_core::time::{CivilDate, Timestamp};
use gunmetal_core::userdata::event::{ProfileId, Stream};
use gunmetal_fs::path::{LogMonth, LogStream};

/// Milliseconds in a day.
const MS_PER_DAY: i64 = 86_400_000;

/// The stream identity the header of a household segment carries. A
/// profile's is its internal ID, which is 128 random bits.
const HOUSEHOLD: [u8; 16] = [0; 16];

/// The data root's name for `stream`.
pub(crate) fn dir(stream: Stream) -> LogStream {
    match stream {
        Stream::Profile(profile) => LogStream::Profile(profile.bytes()),
        Stream::Household => LogStream::Household,
    }
}

/// The stream whose directory the data root listed as `dir`.
pub(crate) fn stream(dir: LogStream) -> Stream {
    match dir {
        LogStream::Profile(id) => Stream::Profile(ProfileId::new(id)),
        LogStream::Household => Stream::Household,
    }
}

/// The month `now` falls in, in UTC.
pub(crate) fn month(now: Timestamp) -> LogMonth {
    // A timestamp is within the years 0000 to 9999, so the date exists and
    // the month has a name; the fallback is never used.
    CivilDate::from_days(now.millis().div_euclid(MS_PER_DAY))
        .ok()
        .and_then(|date| LogMonth::new(date.year(), date.month()))
        .unwrap_or(LogMonth::MIN)
}

/// The header record a segment of `stream` for `month` starts with.
pub(crate) fn header(stream: Stream, month: LogMonth) -> Vec<u8> {
    let id = match stream {
        Stream::Profile(profile) => profile.bytes(),
        Stream::Household => HOUSEHOLD,
    };
    // A month is 1 to 12, so the header is always framed.
    logframe::encode_header(&SegmentHeader {
        stream: id,
        year: month.year(),
        month: month.month(),
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::userlog::testing::{ALICE, frame};
    use gunmetal_fs::path::DataPath;

    fn at(millis: i64) -> Timestamp {
        Timestamp::from_millis(millis).expect("a time in the years 0000 to 9999")
    }

    fn of(year: u16, number: u8) -> LogMonth {
        LogMonth::new(year, number).expect("a month of the calendar")
    }

    #[test]
    fn names_a_profile_s_directory_by_the_bytes_of_its_internal_id() {
        let id = [
            0x00, 0x01, 0x0f, 0x10, 0x7f, 0x80, 0x9a, 0xbc, 0xde, 0xf0, 0xff, 0x12, 0x34, 0x56,
            0x78, 0x90,
        ];
        let profile = Stream::Profile(ProfileId::new(id));
        assert_eq!(dir(profile), LogStream::Profile(id));
        assert_eq!(
            DataPath::log_stream(dir(profile)).rel(),
            "log/p-00010f107f809abcdef0ff1234567890"
        );
        assert_eq!(dir(Stream::Household), LogStream::Household);
        assert_eq!(
            DataPath::log_stream(dir(Stream::Household)).rel(),
            "log/household"
        );
        assert_eq!(stream(LogStream::Profile(id)), profile);
        assert_eq!(stream(LogStream::Household), Stream::Household);
    }

    #[test]
    fn a_month_starts_and_ends_at_midnight_utc() {
        let months: Vec<LogMonth> = [
            1_790_812_799_999,
            1_790_812_800_000,
            1_792_022_400_000,
            1_798_761_599_999,
            1_798_761_600_000,
            -1,
            0,
        ]
        .into_iter()
        .map(|millis| month(at(millis)))
        .collect();
        assert_eq!(
            months,
            [
                of(2026, 9),
                of(2026, 10),
                of(2026, 10),
                of(2026, 12),
                of(2027, 1),
                of(1969, 12),
                of(1970, 1),
            ]
        );
        assert_eq!(month(Timestamp::MIN), of(0, 1));
        assert_eq!(month(Timestamp::MAX), of(9999, 12));
    }

    #[test]
    fn a_header_names_the_stream_the_year_and_the_month() {
        // The stream's 16 octets, 2026 as two little-endian octets, month 10.
        let mut alice = [0xA1; 16].to_vec();
        alice.extend([0xEA, 0x07, 0x0A]);
        assert_eq!(header(ALICE, of(2026, 10)), frame(&alice));
        let mut household = [0x00; 16].to_vec();
        household.extend([0x0F, 0x27, 0x0C]);
        assert_eq!(header(Stream::Household, of(9999, 12)), frame(&household));
    }
}
