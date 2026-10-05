//! Files the probe refuses before any container parser runs, and a probe
//! resumed after its result.

use super::*;
use crate::formats::detect::{DetectError, Format};
use crate::parse::{ParseFault, ReadGuard, SansIo, Step, Window};

/// Verifies: SEC-MED-001
#[test]
fn fails_a_file_whose_content_is_no_format_on_the_allowlist() {
    for (file, ext) in [
        (&b""[..], Some("mp3")),
        (&b"hello, world"[..], Some("mp3")),
        (&b"hello, world"[..], None),
        // A PNG file named as FLAC is not FLAC, and is not read as PNG.
        (&b"\x89PNG\r\n\x1A\n"[..], Some("flac")),
        // An extension outside the allowlist admits nothing.
        (&b"fLaC"[..], Some("txt")),
    ] {
        assert_eq!(
            run(file, ext),
            Err(ProbeError::Detect(DetectError::Unknown { offset: 0 })),
            "{ext:?}"
        );
    }
}

/// A leading tag of 110 octets in a file of 10 hides where the audio
/// starts.
///
/// Verifies: SEC-MED-001
#[test]
fn fails_a_file_whose_leading_tag_runs_past_its_end() {
    assert_eq!(
        run(b"ID3\x04\0\0\0\0\0\x64", Some("mp3")),
        Err(ProbeError::Detect(DetectError::Fault(
            ParseFault::Truncated {
                offset: 0,
                needed: 110,
                available: 10,
            }
        )))
    );
}

#[test]
fn fails_a_picture_lyrics_or_playlist_as_not_audio() {
    let cases = [
        (&b"\xFF\xD8\xFF\xE0"[..], None, Format::Jpeg),
        (&b"\x89PNG\r\n\x1A\n"[..], None, Format::Png),
        (&b"RIFF\x04\0\0\0WEBP"[..], None, Format::Webp),
        (&b"GIF89a"[..], None, Format::Gif),
        (&b"[00:01.00]la"[..], Some("lrc"), Format::Lrc),
        (&b"#EXTM3U"[..], Some("m3u"), Format::M3u),
    ];
    for (file, ext, format) in cases {
        assert_eq!(
            run(file, ext),
            Err(ProbeError::NotAudio { format }),
            "{format:?}"
        );
    }
}

/// A host of its own, so that the probe is still there after its result.
#[test]
fn answers_finished_when_resumed_after_its_result() {
    let file = b"hello, world";
    let limits = Limits::DEFAULT;
    let mut budget = Budget::for_input(0, 0, enough(file));
    let mut probe = probe(Some("mp3"), limits, &mut budget);
    let mut guard = ReadGuard::new(len(file), &limits);
    let mut window = Window::start(len(file));
    let mut resumes = 0;
    let first = loop {
        resumes += 1;
        assert!(resumes < 10);
        match probe.resume(window) {
            Step::Done(outcome) => break outcome,
            Step::Need(request) => {
                assert_eq!(guard.admit(request), Ok(()));
                let start = usize::try_from(request.offset).unwrap();
                let end = start + usize::try_from(request.len).unwrap();
                window = Window {
                    offset: request.offset,
                    bytes: &file[start..end],
                    file_len: len(file),
                };
            }
        }
    };
    assert_eq!(
        first,
        Err(ProbeError::Detect(DetectError::Unknown { offset: 0 }))
    );
    for _ in 0..2 {
        assert_eq!(
            probe.resume(Window::start(len(file))),
            Step::Done(Err(ProbeError::Finished))
        );
    }
}
