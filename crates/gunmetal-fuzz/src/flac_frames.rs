//! The harness for the FLAC frame index in
//! [`gunmetal_core::formats::flac::frames`].

use gunmetal_core::formats::flac::frames::{
    FIXED_STEPS, FlacFrameError, FrameContext, FrameIndex, FrameIndexer, STEPS_PER_OCTET,
};
use gunmetal_core::parse::{Budget, LimitKind, Limits, ParseFault, drive};
use gunmetal_core::values::{BitDepth, SampleRate};

/// What one index of the input reported.
pub type Indexed = Result<FrameIndex, FlacFrameError>;

/// What the frame index reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The whole input indexed as audio, with no STREAMINFO known.
    pub plain: Indexed,
    /// The same, with a STREAMINFO block of 44.1 kHz and 16 bits.
    pub cd: Indexed,
}

/// Indexes all of `data` as audio through [`FrameIndexer::new`] and
/// [`drive`], once knowing no STREAMINFO and once knowing CD audio's, each
/// under the budget the indexer documents.
///
/// # Panics
///
/// Panics when the index breaks an invariant that holds for every input:
/// it differs when the host serves windows of 16 octets instead of one, the
/// host refuses a read, the documented budget runs out, an entry lies
/// outside the input, entries do not strictly increase in offset and first
/// sample, more entries are kept than frames found or than the limit
/// allows, or the stride is not a power of two.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let cd = FrameContext {
        stream_sample_rate: SampleRate::new(44_100).ok(),
        stream_bits: BitDepth::new(16).ok(),
    };
    let plain = FrameContext {
        stream_sample_rate: None,
        stream_bits: None,
    };
    Outcome {
        plain: checked(data, plain),
        cd: checked(data, cd),
    }
}

/// Indexes `data` under `context`, in one window and in windows of 16
/// octets, and checks the invariants.
fn checked(data: &[u8], context: FrameContext) -> Indexed {
    let octets = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let limit = Limits::DEFAULT.get(LimitKind::IndexEntries);
    let windows = Limits::DEFAULT
        .with_override(LimitKind::ReadBytes, 16)
        .unwrap_or(Limits::DEFAULT);
    let whole = index(data, octets, context, &Limits::DEFAULT);
    let windowed = index(data, octets, context, &windows);
    assert!(
        windowed == whole
            && match &whole {
                Ok(found) => {
                    found.entries.windows(2).all(|pair| {
                        pair[0].offset < pair[1].offset
                            && pair[0].first_sample < pair[1].first_sample
                    }) && found
                        .entries
                        .iter()
                        .all(|entry| entry.offset < octets && entry.first_sample < found.end_sample)
                        && u64::try_from(found.entries.len())
                            .is_ok_and(|kept| kept <= found.frames && kept <= limit)
                        && found.stride.is_power_of_two()
                }
                Err(error) => !matches!(
                    error,
                    FlacFrameError::Fault(ParseFault::BudgetExceeded { .. })
                ),
            },
        "{context:?} gave {whole:?}, and {windowed:?} in windows of 16 octets"
    );
    whole
}

/// Indexes all `octets` of `data` under `limits` and the documented budget.
///
/// # Panics
///
/// Panics when the host refuses one of the indexer's reads: the indexer
/// asks only for octets inside the input and within its limits.
fn index(data: &[u8], octets: u64, context: FrameContext, limits: &Limits) -> Indexed {
    let mut budget = Budget::for_input(octets, STEPS_PER_OCTET, FIXED_STEPS);
    drive(
        FrameIndexer::new(context, 0..octets, limits, &mut budget),
        data,
        limits,
    )
    .expect("the host admits every read the indexer asks for")
}
