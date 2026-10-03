//! The harness for the RFC 3339 reader and writer in `gunmetal_core::time`.

use gunmetal_core::time::{self, TimeError, Timestamp};
use gunmetal_core::untrusted::Untrusted;

/// What the RFC 3339 reader reported for one input, read as UTF-8 with
/// each invalid sequence replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    /// [`time::parse_rfc3339`]: milliseconds since the Unix epoch, or why
    /// the text is not a time.
    pub millis: Result<i64, TimeError>,
}

/// Feeds `data` to [`time::parse_rfc3339`], and writes what it accepts
/// with [`time::format_rfc3339`].
///
/// # Panics
///
/// Panics when an accepted time is outside [`Timestamp::MIN`] to
/// [`Timestamp::MAX`], or when it is not written as
/// `YYYY-MM-DDTHH:MM:SS.mmmZ` or does not read back as the same time.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let text = String::from_utf8_lossy(data);
    let parsed = time::parse_rfc3339(Untrusted::new(&text));
    if let Ok(instant) = parsed {
        let written = time::format_rfc3339(instant);
        assert!(
            (Timestamp::MIN..=Timestamp::MAX).contains(&instant)
                && written.len() == 24
                && written.ends_with('Z')
                && time::parse_rfc3339(Untrusted::new(&written)) == Ok(instant),
            "{text:?} read as {instant:?}, written {written:?}"
        );
    }
    Outcome {
        millis: parsed.map(Timestamp::millis),
    }
}
