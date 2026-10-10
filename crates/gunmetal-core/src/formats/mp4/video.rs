//! A video sample entry (ISO/IEC 14496-12, section 12.1.3).
//!
//! This reads the fixed fields of one `VisualSampleEntry`: the coding
//! name, the width and the height. Codec configuration boxes that follow
//! those fields stay unread, for the remuxer to take next. A width or
//! height of zero is refused. Every other input returns a typed error
//! (SEC-MED-001).

use std::num::NonZeroU16;

use super::boxes::FourCc;
use super::probe::Mp4Error;
use crate::parse::Cursor;

/// Octets to skip to reach the width.
const BEFORE_WIDTH: u64 = 24;
/// Octets after the height through the end of the fixed fields.
const AFTER_HEIGHT: u64 = 50;

/// The picture size and coding name of one video sample entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoSample {
    /// The coding name, such as `avc1` or `hvc1`.
    pub format: FourCc,
    /// The width in pixels. Never zero.
    pub width: NonZeroU16,
    /// The height in pixels. Never zero.
    pub height: NonZeroU16,
}

/// Reads the fixed fields of a video sample entry body.
///
/// `body` is the sample entry after its box header. `offset` is the file
/// offset of `body`'s first octet. Bytes after the fixed fields are left
/// unread.
///
/// # Errors
///
/// [`Mp4Error::Fault`] when the fixed fields are cut short, and
/// [`Mp4Error::Unexpected`] when the width or the height is zero.
pub fn video_sample(format: FourCc, body: &[u8], offset: u64) -> Result<VideoSample, Mp4Error> {
    let mut cursor = Cursor::at(body, offset);
    cursor.skip(BEFORE_WIDTH)?;
    let width_at = cursor.offset();
    let width_raw = cursor.u16_be()?;
    let width = NonZeroU16::new(width_raw).ok_or(Mp4Error::Unexpected {
        kind: format,
        offset: width_at,
        found: u64::from(width_raw),
    })?;
    let height_at = cursor.offset();
    let height_raw = cursor.u16_be()?;
    let height = NonZeroU16::new(height_raw).ok_or(Mp4Error::Unexpected {
        kind: format,
        offset: height_at,
        found: u64::from(height_raw),
    })?;
    cursor.skip(AFTER_HEIGHT)?;
    Ok(VideoSample {
        format,
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::fault::ParseFault;
    use proptest::prelude::*;

    /// The stack size SEC-MED-001 names, in octets.
    const STACK: usize = 262_144;

    /// Runs `work` on a fresh thread with a 256 KiB stack, so a parse that
    /// recursed without a bound would fail this test instead of passing on
    /// the runner's larger stack.
    fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(STACK)
            .spawn(work)
            .expect("the test thread starts")
            .join()
            .expect("the code under test returned instead of panicking")
    }

    fn sample(width: u16, height: u16, tail: &[u8]) -> Vec<u8> {
        let mut body = [0_u8; 78];
        body[24..26].copy_from_slice(&width.to_be_bytes());
        body[26..28].copy_from_slice(&height.to_be_bytes());
        let mut bytes = body.to_vec();
        bytes.extend_from_slice(tail);
        bytes
    }

    #[test]
    fn reads_the_picture_size_and_leaves_the_codec_box() {
        let body = sample(1920, 1080, b"avcC");
        assert_eq!(
            video_sample(FourCc(*b"avc1"), &body, 80),
            Ok(VideoSample {
                format: FourCc(*b"avc1"),
                width: NonZeroU16::new(1920).expect("non-zero"),
                height: NonZeroU16::new(1080).expect("non-zero"),
            })
        );
    }

    #[test]
    fn refuses_a_zero_width() {
        assert_eq!(
            video_sample(FourCc(*b"hvc1"), &sample(0, 1080, b""), 10),
            Err(Mp4Error::Unexpected {
                kind: FourCc(*b"hvc1"),
                offset: 34,
                found: 0,
            })
        );
    }

    #[test]
    fn refuses_a_zero_height() {
        assert_eq!(
            video_sample(FourCc(*b"avc1"), &sample(1920, 0, b""), 10),
            Err(Mp4Error::Unexpected {
                kind: FourCc(*b"avc1"),
                offset: 36,
                found: 0,
            })
        );
    }

    #[test]
    fn refuses_a_body_cut_before_the_width() {
        assert_eq!(
            video_sample(FourCc(*b"avc1"), &[0; 10], 4),
            Err(Mp4Error::Fault(ParseFault::Truncated {
                offset: 4,
                needed: 24,
                available: 10,
            }))
        );
    }

    #[test]
    fn refuses_a_body_cut_on_the_width() {
        assert_eq!(
            video_sample(FourCc(*b"avc1"), &[0; 25], 0),
            Err(Mp4Error::Fault(ParseFault::Truncated {
                offset: 24,
                needed: 2,
                available: 1,
            }))
        );
    }

    #[test]
    fn refuses_a_body_cut_on_the_height() {
        let mut body = [0_u8; 27];
        body[24..26].copy_from_slice(&1920_u16.to_be_bytes());
        assert_eq!(
            video_sample(FourCc(*b"avc1"), &body, 0),
            Err(Mp4Error::Fault(ParseFault::Truncated {
                offset: 26,
                needed: 2,
                available: 1,
            }))
        );
    }

    #[test]
    fn refuses_a_body_cut_after_the_size() {
        let mut body = [0_u8; 30];
        body[24..26].copy_from_slice(&1280_u16.to_be_bytes());
        body[26..28].copy_from_slice(&720_u16.to_be_bytes());
        assert_eq!(
            video_sample(FourCc(*b"hev1"), &body, 100),
            Err(Mp4Error::Fault(ParseFault::Truncated {
                offset: 128,
                needed: 50,
                available: 2,
            }))
        );
    }

    proptest! {
        /// Verifies: SEC-MED-001
        #[test]
        fn returns_for_any_bytes(body in proptest::collection::vec(any::<u8>(), 0..128)) {
            let format = FourCc(*b"avc1");
            let result = on_small_stack(move || video_sample(format, &body, 0));
            match result {
                Ok(sample) => prop_assert_eq!(sample.format, format),
                Err(Mp4Error::Fault(ParseFault::Truncated { .. }) | Mp4Error::Unexpected { found: 0, .. }) => {}
                Err(other) => prop_assert!(false, "{other:?}"),
            }
        }
    }
}
