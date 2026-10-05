//! The harness for the file probe in [`gunmetal_core::probe`].

use gunmetal_core::parse::{Budget, Limits, drive};
use gunmetal_core::probe::{FIXED_STEPS, ProbeError, Probed, STEPS_PER_OCTET, probe};

/// What the probe reported for one input.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// The file is audio the probe reads.
    Found(Box<Probed>),
    /// The file could not be probed.
    Error(ProbeError),
}

/// Feeds `data` to [`probe`] as a file with no extension, so that every
/// format with a signature is a candidate, under the default limits and
/// the budget the probe documents as enough for any file of this length.
///
/// # Panics
///
/// Panics when the probe breaks an invariant that holds for every input: a
/// read the host must refuse, or an audio window that runs past the end of
/// the file.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let len = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let limits = Limits::DEFAULT;
    let mut budget = Budget::for_input(len, STEPS_PER_OCTET, FIXED_STEPS);
    let answer = drive(probe(None, limits, &mut budget), data, &limits)
        .expect("the probe asked for a refused read");
    match answer {
        Ok(probed) => {
            let window = probed.facts.identity.audio_window;
            assert!(window.end() <= len, "{window:?} runs past {len} octets");
            Outcome::Found(Box::new(probed))
        }
        Err(error) => Outcome::Error(error),
    }
}
