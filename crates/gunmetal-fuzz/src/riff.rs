//! The harness for the WAV parser in [`gunmetal_core::formats::riff`].

use gunmetal_core::formats::riff::{self, ByteRange, RiffError, Wav, WavFile};
use gunmetal_core::parse::{Budget, Limits, ParseFault, drive};
use gunmetal_core::problem::Describe;

/// Feeds `data` to [`Wav`] as a whole file, under the default limits and
/// the step budget the parser documents as always enough.
///
/// # Panics
///
/// Panics when the parser breaks an invariant that holds for every input:
/// a read the host refuses, a range that ends past the end of the file, a
/// budget spent although it was the one the parser documents, or an error
/// that gives no reason.
///
/// # Errors
///
/// What the parser reports for a file it cannot play.
pub fn run(data: &[u8]) -> Result<WavFile, RiffError> {
    let len = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let budget = Budget::for_input(len, riff::STEPS_PER_OCTET, riff::STEPS_FIXED);
    let outcome = drive(Wav::new(&Limits::DEFAULT, budget), data, &Limits::DEFAULT)
        .expect("the parser asks only for octets inside the file, a few at a time");
    let inside = |range: ByteRange| {
        range
            .offset
            .checked_add(range.len)
            .is_some_and(|end| end <= len)
    };
    assert!(
        match &outcome {
            Ok(file) =>
                inside(file.data)
                    && file.info.is_none_or(inside)
                    && file.id3.is_none_or(inside)
                    && !matches!(file.stopped, Some(ParseFault::BudgetExceeded { .. })),
            Err(error) =>
                !matches!(error, RiffError::Fault(ParseFault::BudgetExceeded { .. }))
                    && !error.problem().args.is_empty(),
        },
        "{len} octets gave {outcome:?}"
    );
    outcome
}
