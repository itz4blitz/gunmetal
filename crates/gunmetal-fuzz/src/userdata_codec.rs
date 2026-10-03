//! The harness for the user-event reader and writer in
//! `gunmetal_core::userdata::codec`.

use gunmetal_core::parse::{Budget, ParseFault};
use gunmetal_core::untrusted::Untrusted;
use gunmetal_core::userdata::codec::{self, EventError, FIXED_STEPS, STEPS_PER_BYTE};
use gunmetal_core::userdata::event::Event;

/// What the event reader reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// [`codec::decode`], with the budget it documents as enough.
    pub event: Result<Event, EventError>,
}

/// Feeds `data` to [`codec::decode`], and writes what it accepts with
/// [`codec::encode`].
///
/// # Panics
///
/// Panics when an accepted event does not encode as exactly `data` again,
/// since every event has one encoding, or when the decode ran out of the
/// budget it documents as enough.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let len = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let mut budget = Budget::for_input(len, STEPS_PER_BYTE, FIXED_STEPS);
    let event = codec::decode(Untrusted::new(data), &mut budget);
    let written = event.as_ref().ok().map(codec::encode);
    assert!(written.is_none_or(|written| written == data));
    assert!(!matches!(
        event,
        Err(EventError::Fault(ParseFault::BudgetExceeded { .. }))
    ));
    Outcome { event }
}
