//! The harness for the `Range` header parser in
//! `gunmetal_core::http::range` (SEC-NET-050, SEC-MED-060).

use gunmetal_core::http::range::{Ignored, RangeOutcome, Refusal, range};
use gunmetal_core::untrusted::Untrusted;

/// The representation lengths every input is judged against: empty, one
/// octet, a small file, and the longest length there is.
pub const LENGTHS: [u64; 4] = [0, 1, 1_000, u64::MAX];

/// What the parser answered for one length, as a value the replay tests
/// can write out. Only the parser can make a `ByteRange`, so a served
/// range is given by its ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// The whole representation, with 200.
    Whole(Ignored),
    /// The octets from `first` to `last`, with 206.
    Partial {
        /// The first octet served.
        first: u64,
        /// The last octet served.
        last: u64,
    },
    /// 416 Range Not Satisfiable.
    NotSatisfiable(Refusal),
}

/// Feeds `data`, as a `Range` header, to [`range`] once for each of
/// [`LENGTHS`], and returns the answers in that order.
///
/// # Panics
///
/// Panics when a range the parser would serve does not lie inside the
/// representation, or reports a length its ends do not give.
#[must_use]
pub fn run(data: &[u8]) -> [Answer; 4] {
    LENGTHS.map(|len| match range(Untrusted::new(data), len) {
        RangeOutcome::Whole(ignored) => Answer::Whole(ignored),
        RangeOutcome::NotSatisfiable(refusal) => Answer::NotSatisfiable(refusal),
        RangeOutcome::Partial(part) => {
            let octets = part
                .last()
                .checked_sub(part.first())
                .and_then(|span| span.checked_add(1));
            assert!(
                part.last() < len && octets == Some(part.octets()),
                "{part:?} of {len}"
            );
            Answer::Partial {
                first: part.first(),
                last: part.last(),
            }
        }
    })
}
