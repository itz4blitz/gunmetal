//! The harness for the MPEG audio frame header and stream parsers in
//! [`gunmetal_core::formats::mpa`].

use gunmetal_core::formats::mpa::{
    FIXED_STEPS, FrameHeader, HeaderError, Layer, MpaError, MpegStream, STEPS_PER_OCTET,
    frame_header, stream_info,
};
use gunmetal_core::parse::{Budget, Limits, ParseFault, drive};

/// What the MPEG audio parsers reported for one input.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// [`frame_header`] of the first four octets, when the input holds them.
    pub header: Option<Result<FrameHeader, HeaderError>>,
    /// [`stream_info`] driven over the whole input.
    pub stream: Result<MpegStream, MpaError>,
}

/// Feeds `data` to [`frame_header`] and [`stream_info`].
///
/// # Panics
///
/// Panics when the parser breaks an invariant that holds for every input:
/// a read the driver refuses, a step budget the parser documents as enough
/// running out, an accepted stream that is not Layer III, a seek index that
/// is not in order, or a first-frame length below the smallest Layer III
/// frame.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let header = data.first_chunk().map(|&octets| frame_header(octets));
    let file_len = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let limits = Limits::DEFAULT;
    let stream = drive(
        stream_info(
            &limits,
            Budget::for_input(file_len, STEPS_PER_OCTET, FIXED_STEPS),
        ),
        data,
        &limits,
    )
    .expect("parser requested a read the guard refuses");
    if let Ok(stream) = &stream {
        assert!(
            stream.header.layer == Layer::III
                && stream.header.len >= 24
                && stream.start <= stream.end
                && stream.seek.stride.is_power_of_two()
                && stream.seek.points.windows(2).all(|window| {
                    matches!(
                        window,
                        [first, second]
                            if first.sample <= second.sample && first.offset <= second.offset
                    )
                }),
            "{stream:?} breaks a stream invariant"
        );
    }
    assert!(
        !matches!(
            stream,
            Err(MpaError::Fault(ParseFault::BudgetExceeded { .. }))
        ),
        "the documented budget was not enough for {file_len} octets"
    );
    Outcome { header, stream }
}
