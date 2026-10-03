//! The harness for the APE tag parser in [`gunmetal_core::formats::ape`].

use gunmetal_core::formats::ape::{self, ApeError, ApeTag, ApeValue, ItemProblem};
use gunmetal_core::parse::{Budget, LimitKind, Limits, Window};

/// Octets the tail case puts before the input.
pub const BEFORE: u64 = 1_000;

/// What the APE parser reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// [`ape::parse_ape`] over the input as a whole file.
    pub whole: Result<Option<ApeTag>, ApeError>,
    /// The same over the input as the last octets of a file [`BEFORE`]
    /// octets longer.
    pub tail: Result<Option<ApeTag>, ApeError>,
    /// The steps of budget each reading spent: the whole file's, then the
    /// tail's.
    pub steps: [u64; 2],
}

/// Feeds `data` to [`ape::parse_ape`] as a whole file, and as the last
/// octets of a file [`BEFORE`] octets longer, and counts the steps each
/// reading spends.
///
/// # Panics
///
/// Panics when the parser breaks an invariant that holds for every input:
/// more steps than the 2 + n / 9 it documents for n octets, an error or
/// problem offset outside the file or the tag, a tag that neither ends the
/// file nor ends just before its last 128 octets, more items than the
/// tag-field limit, a key the specification forbids, a range outside the
/// tag, text holding a control other than tab and line feed or longer than
/// the long-text limit, or a problem after the one that stopped the
/// reading.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let len = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let file_len = BEFORE.saturating_add(len);
    let (whole, whole_steps) = parse(Window {
        offset: 0,
        bytes: data,
        file_len: len,
    });
    let (tail, tail_steps) = parse(Window {
        offset: BEFORE,
        bytes: data,
        file_len,
    });
    let fields = usize::try_from(Limits::DEFAULT.get(LimitKind::TagFields)).unwrap_or(usize::MAX);
    let long_text = usize::try_from(Limits::DEFAULT.get(LimitKind::LongText)).unwrap_or(usize::MAX);
    for (found, steps, end) in [(&whole, whole_steps, len), (&tail, tail_steps, file_len)] {
        assert!(
            steps.saturating_sub(2).saturating_mul(9) <= len,
            "{steps} steps for {len} octets"
        );
        if let Err(error) = found {
            assert!(error.offset() < end, "{error:?} in a file of {end} octets");
        }
        if let Ok(Some(tag)) = found {
            let range = &tag.range;
            assert!(
                range.start.saturating_add(32) <= range.end
                    && (range.end == end || range.end == end.saturating_sub(128))
                    && tag.items.len() <= fields,
                "{tag:?} in a file of {end} octets"
            );
            for item in &tag.items {
                assert!(
                    (2..=255).contains(&item.key.len())
                        && item.key.bytes().all(|octet| (0x20..=0x7E).contains(&octet))
                        && match &item.value {
                            ApeValue::Binary(inner) | ApeValue::Reserved(inner) => {
                                range.start <= inner.start
                                    && inner.start <= inner.end
                                    && inner.end <= range.end
                            }
                            ApeValue::Text(values) | ApeValue::Locator(values) => {
                                values.iter().all(|value| {
                                    value.value.len() <= long_text
                                        && value
                                            .value
                                            .chars()
                                            .all(|c| !c.is_control() || c == '\t' || c == '\n')
                                })
                            }
                        },
                    "{item:?} in {range:?}"
                );
            }
            // Only the last problem may have stopped the reading; every
            // other one is a skipped item.
            assert!(
                tag.problems
                    .iter()
                    .rev()
                    .skip(1)
                    .all(|problem| matches!(problem, ItemProblem::BadKey { .. }))
                    && tag
                        .problems
                        .iter()
                        .all(|problem| range.contains(&problem.offset())),
                "{tag:?}"
            );
        }
    }
    Outcome {
        whole,
        tail,
        steps: [whole_steps, tail_steps],
    }
}

/// [`ape::parse_ape`] over `window` under the default limits, and the steps
/// of budget it spent.
fn parse(window: Window<'_>) -> (Result<Option<ApeTag>, ApeError>, u64) {
    let mut budget = Budget::for_input(0, 0, u64::MAX);
    let found = ape::parse_ape(window, &Limits::DEFAULT, &mut budget);
    (found, u64::MAX.saturating_sub(budget.remaining()))
}
