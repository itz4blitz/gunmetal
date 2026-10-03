//! The harness for the AIFF parser in [`gunmetal_core::formats::aiff`].

use gunmetal_core::formats::aiff::{self, Aiff, AiffError, AiffFile};
use gunmetal_core::formats::riff::ByteRange;
use gunmetal_core::parse::{Budget, Limits, ParseFault, drive};
use gunmetal_core::problem::Describe;

/// Feeds `data` to [`Aiff`] as a whole file, under the default limits and
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
pub fn run(data: &[u8]) -> Result<AiffFile, AiffError> {
    let len = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let budget = Budget::for_input(len, aiff::STEPS_PER_OCTET, aiff::STEPS_FIXED);
    let outcome = drive(Aiff::new(&Limits::DEFAULT, budget), data, &Limits::DEFAULT)
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
                inside(file.sound)
                    && file.id3.is_none_or(inside)
                    && !matches!(file.stopped, Some(ParseFault::BudgetExceeded { .. })),
            Err(error) =>
                !matches!(error, AiffError::Fault(ParseFault::BudgetExceeded { .. }))
                    && !error.problem().args.is_empty(),
        },
        "{len} octets gave {outcome:?}"
    );
    outcome
}
