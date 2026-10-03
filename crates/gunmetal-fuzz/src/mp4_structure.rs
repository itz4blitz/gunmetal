//! The structure-aware harness for MP4 (SEC-MED-031): valid framing around
//! mutated fields.
//!
//! It writes the sample tables of a track its input describes and changes
//! one octet of them, through [`mp4_sample_table::structured`]. The box
//! walker's own generator, for the box tree, the audio sample entries and
//! the item lists, joins it here.

use crate::mp4_sample_table::{self, Outcome};

/// Runs the MP4 structure-aware generators on `data`.
///
/// # Errors
///
/// Returns the parser's error for what the generators wrote, which the
/// harness reports rather than treating as a finding.
///
/// # Panics
///
/// Panics when a generator's invariants break; see
/// [`mp4_sample_table::structured`].
pub fn run(data: &[u8]) -> Outcome {
    mp4_sample_table::structured(data)
}
