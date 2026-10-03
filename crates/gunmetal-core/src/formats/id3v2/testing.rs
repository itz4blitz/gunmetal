//! Test support shared by the `ID3v2` tests.

use crate::parse::{Budget, Limits, ParseFault};
use crate::text::Text;

use super::{
    BUDGET_FIXED, BUDGET_PER_OCTET, Frame, FrameBody, FrameId, Id3v2Error, Id3v2Tag, Span,
    TagProblem, parse,
};

/// The stack size SEC-MED-001 names for its property tests, in octets.
const SMALL_STACK: usize = 262_144;

/// Runs `work` on a fresh thread with a 256 KiB stack, so a parse that
/// recurses too deeply fails its test.
pub(super) fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(SMALL_STACK)
        .spawn(work)
        .expect("the test thread starts")
        .join()
        .expect("the code under test returned instead of panicking")
}

/// Reads `tag` under the default limits with the budget the module
/// documents for its length.
pub(super) fn read(tag: &[u8]) -> Result<Id3v2Tag, Id3v2Error> {
    read_with(tag, &Limits::DEFAULT)
}

/// Reads `tag` under `limits` with the budget the module documents for its
/// length.
pub(super) fn read_with(tag: &[u8], limits: &Limits) -> Result<Id3v2Tag, Id3v2Error> {
    let len = u64::try_from(tag.len()).expect("a test tag fits u64");
    parse(
        tag,
        limits,
        &mut Budget::for_input(len, BUDGET_PER_OCTET, BUDGET_FIXED),
    )
}

/// The frames and problems of `tag`, which must be readable.
pub(super) fn contents(tag: &[u8]) -> (Vec<Frame>, Vec<TagProblem>) {
    let read = read(tag).expect("the tag is readable");
    (read.frames, read.problems)
}

/// The steps a parse of `tag` under `limits` charges.
pub(super) fn steps(tag: &[u8], limits: &Limits) -> u64 {
    let mut budget = Budget::for_input(0, 0, u64::MAX);
    let _ = parse(tag, limits, &mut budget);
    u64::MAX.saturating_sub(budget.remaining())
}

/// The default limits with `kind` set to `value`.
pub(super) fn limits(kind: crate::parse::LimitKind, value: u64) -> Limits {
    Limits::DEFAULT
        .with_override(kind, value)
        .expect("the override is below the ceiling")
}

/// A frame identifier of three or four octets.
pub(super) fn id(octets: &[u8]) -> FrameId {
    if let [a, b, c] = *octets {
        return FrameId::Three([a, b, c]);
    }
    FrameId::Four(
        octets
            .try_into()
            .expect("a frame ID has three or four octets"),
    )
}

/// A frame as the parser should report it.
pub(super) fn frame(octets: &[u8], offset: u64, flags: u16, body: FrameBody) -> Frame {
    Frame {
        id: id(octets),
        offset,
        flags,
        body,
    }
}

/// A body kept raw, stored as it is read.
pub(super) fn raw(start: u64, end: u64) -> FrameBody {
    FrameBody::Raw(span(start, end, false))
}

/// A span of the tag.
pub(super) fn span(start: u64, end: u64, unsynchronised: bool) -> Span {
    Span {
        start,
        end,
        unsynchronised,
    }
}

/// Text that was neither capped nor repaired.
pub(super) fn text(value: &str) -> Text {
    Text {
        value: value.to_owned(),
        truncated: false,
        replaced: false,
    }
}

/// Several texts that were neither capped nor repaired.
pub(super) fn texts(values: &[&str]) -> Vec<Text> {
    values.iter().map(|value| text(value)).collect()
}

/// A text frame body of `values`.
pub(super) fn text_body(values: &[&str]) -> FrameBody {
    FrameBody::Text(texts(values))
}

/// The problem a parse records for a frame larger than what is left.
pub(super) fn truncated(offset: u64, needed: u64, available: u64) -> TagProblem {
    TagProblem::Fault(ParseFault::Truncated {
        offset,
        needed,
        available,
    })
}
