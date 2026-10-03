//! The harness for the FLAC metadata parser in
//! [`gunmetal_core::formats::flac::metadata`].

use gunmetal_core::formats::flac::metadata::{
    FlacError, FlacMetadata, STEPS_FIXED, STEPS_PER_OCTET, parse_metadata,
};
use gunmetal_core::parse::{Budget, Limits, drive};

/// Octets of the marker, a block header and STREAMINFO, which every stream
/// starts with.
pub const STREAM_INFO_END: u64 = 42;

/// What the parser reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The metadata, or why there is none.
    pub parsed: Result<FlacMetadata, FlacError>,
    /// The steps of its budget the parse spent.
    pub steps: u64,
}

/// Parses `data` as a FLAC stream that starts at its first octet, with
/// [`parse_metadata`] driven over it in memory, under the default limits
/// and the budget the parser's documentation says no parse can spend.
///
/// # Panics
///
/// Panics when the parser breaks an invariant that holds for every input:
/// it asks for a read the host refuses, or spends more than one step per
/// four octets; its metadata puts the audio past the input or before the
/// end of STREAMINFO, gives a byte range outside the metadata, keeps seek
/// points out of order or a placeholder, or keeps more than 16 pictures;
/// or an error points past the input.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let len = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let limits = Limits::DEFAULT;
    let mut budget = Budget::for_input(len, STEPS_PER_OCTET, STEPS_FIXED);
    let driven = drive(parse_metadata(0, &limits, &mut budget), data, &limits);
    let parsed = driven.expect("the parser asks for no read the host refuses");
    let steps = len
        .saturating_mul(STEPS_PER_OCTET)
        .saturating_add(STEPS_FIXED)
        - budget.remaining();
    assert!(
        steps.saturating_mul(4) <= len,
        "{steps} steps for {len} octets"
    );
    match &parsed {
        Ok(found) => {
            let audio = found.audio_start;
            assert!(
                (STREAM_INFO_END..=len).contains(&audio),
                "the audio starts at {audio} of {len} octets"
            );
            let ranges = found
                .comment
                .iter()
                .chain(found.pictures.iter().map(|picture| &picture.data))
                .chain(found.raw.iter().map(|raw| &raw.body));
            for range in ranges {
                assert!(
                    STREAM_INFO_END <= range.start
                        && range.start <= range.end
                        && range.end <= audio,
                    "{range:?} lies outside the metadata, which ends at {audio}"
                );
            }
            let points = &found.seek_table;
            assert!(
                points
                    .windows(2)
                    .all(|pair| pair[0].sample < pair[1].sample)
                    && points.iter().all(|point| point.sample < u64::MAX),
                "seek points out of order: {points:?}"
            );
            let pictures = found.pictures.len();
            assert!(pictures <= 16, "{pictures} pictures");
        }
        Err(error) => assert!(error.offset() <= len, "{error:?} lies past {len} octets"),
    }
    Outcome { parsed, steps }
}
