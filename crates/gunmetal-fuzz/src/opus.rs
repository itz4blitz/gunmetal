//! The harness for the Opus stream headers in
//! [`gunmetal_core::formats::opus`].

use gunmetal_core::formats::opus::{self, OpusError, OpusHead};
use gunmetal_core::parse::{Budget, Depth, Limits};

/// What the Opus header parsers reported for one input.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome<'a> {
    /// [`opus::opus_head`] applied to the whole input.
    pub head: Result<OpusHead, OpusError>,
    /// [`opus::opus_tags`] applied to the whole input.
    pub tags: Result<&'a [u8], OpusError>,
}

/// Feeds `data` to [`opus::opus_head`] and [`opus::opus_tags`] under the
/// default limits, at the top level of a container, each with a budget no
/// input can spend.
///
/// # Panics
///
/// Panics when a parser breaks an invariant that holds for every input:
/// more steps than its documented bound (one for the identification
/// header, a quarter of the input's length plus two for the comment
/// header), an error offset past the end of the input, or a comment block
/// that is not the octets right after the signature.
#[must_use]
pub fn run(data: &[u8]) -> Outcome<'_> {
    let octets = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let mut budget = Budget::for_input(0, 0, u64::MAX);
    let head = opus::opus_head(data, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT);
    assert_eq!(
        u64::MAX - budget.remaining(),
        1,
        "opus_head spent other than one step"
    );
    let mut budget = Budget::for_input(0, 0, u64::MAX);
    let tags = opus::opus_tags(data, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT);
    assert!(
        u64::MAX - budget.remaining() <= octets / 4 + 2,
        "opus_tags spent more than a quarter of {octets} octets plus two steps, {budget:?} left"
    );
    for error in [head.as_ref().err(), tags.as_ref().err()]
        .into_iter()
        .flatten()
    {
        assert!(
            error.offset() <= octets,
            "{error:?} lies outside {octets} octets"
        );
    }
    if let Ok(block) = tags {
        assert!(
            data.get(8..).is_some_and(|rest| rest.starts_with(block)),
            "opus_tags returned {block:02X?}, which does not follow the signature"
        );
    }
    Outcome { head, tags }
}
