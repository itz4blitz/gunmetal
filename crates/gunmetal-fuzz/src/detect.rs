//! The harness for format detection in
//! [`gunmetal_core::formats::detect`] (SEC-MED-011, SEC-HIS-017, SEC-HIS-036).

use gunmetal_core::formats::detect::{self, DetectError, Detected, Detector, Format};
use gunmetal_core::parse::{Limits, ReadRequest, SansIo, Step, Window, drive};

/// The audio formats.
const AUDIO: &[Format] = &[
    Format::Flac,
    Format::Mpeg,
    Format::Mp4,
    Format::Ogg,
    Format::Wav,
    Format::Aiff,
];

/// The image formats.
const IMAGES: &[Format] = &[Format::Jpeg, Format::Png, Format::Webp, Format::Gif];

/// The formats with a binary signature.
const BINARY: &[Format] = &[
    Format::Flac,
    Format::Mpeg,
    Format::Mp4,
    Format::Ogg,
    Format::Wav,
    Format::Aiff,
    Format::Jpeg,
    Format::Png,
    Format::Webp,
    Format::Gif,
];

/// The formats that may follow a leading `ID3v2` tag.
const AFTER_TAGS: &[Format] = &[Format::Flac, Format::Mpeg];

/// The extension hints the first octet of an input chooses from, by its
/// value modulo their number, each with the formats it admits: no hint,
/// names of each kind, two of them in capitals, and a name outside the
/// allowlist.
pub const HINTS: [(Option<&str>, &[Format]); 8] = [
    (None, BINARY),
    (Some("flac"), AUDIO),
    (Some("MP3"), AUDIO),
    (Some("m4a"), AUDIO),
    (Some("JPG"), IMAGES),
    (Some("lrc"), &[Format::Lrc]),
    (Some("m3u"), &[Format::M3u]),
    (Some("txt"), &[]),
];

/// The most reads one detection makes: the start of the file and what
/// follows each of up to seven leading `ID3v2` tags.
pub const MAX_READS: usize = 8;

/// The most octets one read asks for.
pub const MAX_READ: u32 = 512;

/// What detection reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The extension hint the input's first octet chose.
    pub hint: Option<&'static str>,
    /// Every read detection asked for, in order.
    pub reads: Vec<ReadRequest>,
    /// What detection returned.
    pub result: Result<Detected, DetectError>,
}

/// A detector that records each read it asks for, and checks each against
/// the ones before.
struct Recorder {
    /// The detector.
    detector: Detector,
    /// Its reads so far.
    reads: Vec<ReadRequest>,
}

impl SansIo for Recorder {
    type Output = (Vec<ReadRequest>, Result<Detected, DetectError>);

    fn resume(&mut self, window: Window<'_>) -> Step<Self::Output> {
        match self.detector.resume(window) {
            Step::Need(request) => {
                assert!(
                    self.reads.len() < MAX_READS
                        && self
                            .reads
                            .last()
                            .map_or(request.offset == 0, |last| { request.offset > last.offset })
                        && (1..=MAX_READ).contains(&request.len),
                    "read {request:?} after {:?}",
                    self.reads
                );
                self.reads.push(request);
                Step::Need(request)
            }
            Step::Done(result) => Step::Done((std::mem::take(&mut self.reads), result)),
        }
    }
}

/// Detects the format of the input after its first octet, which chooses the
/// extension hint from [`HINTS`]. An empty input is an empty file with no
/// hint.
///
/// # Panics
///
/// Panics when detection breaks an invariant that holds for every input: a
/// read outside the file, not after the one before, of more than
/// [`MAX_READ`] octets, or more than [`MAX_READS`] reads; a format the hint
/// does not admit; a format whose content does not start where the last
/// read did, or that follows a leading tag and is neither FLAC nor MP3; or
/// an offset past the end of the file.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let ((hint, admitted), file) = data
        .split_first()
        .map_or((HINTS[0], data), |(&choice, file)| {
            (HINTS[usize::from(choice) % HINTS.len()], file)
        });
    let file_len = u64::try_from(file.len()).unwrap_or(u64::MAX);
    let recorder = Recorder {
        detector: detect::detect(hint),
        reads: Vec::new(),
    };
    let (reads, result) =
        drive(recorder, file, &Limits::DEFAULT).expect("detection reads only inside its file");
    match result {
        Ok(Detected { format, start }) => assert!(
            admitted.contains(&format)
                && reads.last().is_some_and(|last| last.offset == start)
                && (start == 0 || (file.starts_with(b"ID3") && AFTER_TAGS.contains(&format))),
            "{hint:?} found {format:?} at {start} after {reads:?}"
        ),
        Err(DetectError::Unknown { offset }) => assert!(
            offset <= file_len,
            "unknown at {offset} in {file_len} octets"
        ),
        Err(DetectError::Fault(fault)) => {
            assert!(fault.offset() < file_len, "{fault:?} in {file_len} octets");
        }
    }
    Outcome {
        hint,
        reads,
        result,
    }
}
