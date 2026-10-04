//! The harness for the MP4 audio probe in [`gunmetal_core::formats::mp4`].

use gunmetal_core::formats::mp4::{FIXED_STEPS, Mp4Audio, Mp4Error, Probe, STEPS_PER_BYTE};
use gunmetal_core::parse::{Budget, Limits, drive};

/// What the probe reported for one input.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// The file held an audio track.
    Audio(Box<Mp4Audio>),
    /// The file could not be read as MP4 audio.
    Error(Mp4Error),
}

/// Feeds `data` to [`Probe`] under the default limits and the budget the
/// probe documents as enough for any file of this length.
///
/// # Panics
///
/// Panics when the probe breaks an invariant that holds for every input: a
/// read the host must refuse, or an error whose offset lies past the end of
/// the file.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let len = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let limits = Limits::DEFAULT;
    let parsed = drive(
        Probe::new(limits, Budget::for_input(len, STEPS_PER_BYTE, FIXED_STEPS)),
        data,
        &limits,
    )
    .expect("the probe asked for a refused read");
    match parsed {
        Ok(audio) => Outcome::Audio(Box::new(audio)),
        Err(error) => {
            assert!(error.offset() <= len, "{error:?} lies past {len} octets");
            Outcome::Error(error)
        }
    }
}
