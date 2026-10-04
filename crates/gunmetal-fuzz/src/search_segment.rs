//! The harness for search-index segments in
//! `gunmetal_core::search::segment`, which a server may ship to a device
//! (API-SYNC-07).

use gunmetal_core::parse::{Budget, ParseFault};
use gunmetal_core::search::{FIXED_STEPS, Hit, Index, IndexError, KindFilter, STEPS_PER_OCTET};

/// The query every index that reads is asked, so that a replay test can pin
/// what it finds.
pub const PROBE: &str = "go";

/// The most hits of one type any query here may return.
pub const LIMIT: u16 = 8;

/// What the segment reader reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The hits for [`PROBE`] in the index [`Index::from_bytes`] read, or
    /// why it read none.
    pub hits: Result<Vec<Hit>, IndexError>,
}

/// Feeds `data` to [`Index::from_bytes`] under its documented budget, and
/// writes and queries the index it reads.
///
/// # Panics
///
/// Panics when the documented budget runs out, when an index that was read
/// does not write back as exactly `data`, so that a segment has more than
/// one written form, or when a query returns more than [`LIMIT`] hits of
/// each of the four types.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let mut budget = Budget::for_input(
        u64::try_from(data.len()).unwrap_or(u64::MAX),
        STEPS_PER_OCTET,
        FIXED_STEPS,
    );
    let index = Index::from_bytes(data, &mut budget);
    assert!(!matches!(
        index,
        Err(IndexError::Fault(ParseFault::BudgetExceeded { .. }))
    ));
    let most = usize::from(LIMIT).saturating_mul(4);
    Outcome {
        hits: index.map(|index| {
            assert_eq!(index.to_bytes(), data);
            let own_text = String::from_utf8_lossy(data);
            assert!(index.query(&own_text, KindFilter::All, LIMIT).len() <= most);
            let hits = index.query(PROBE, KindFilter::All, LIMIT);
            assert!(hits.len() <= most);
            hits
        }),
    }
}
