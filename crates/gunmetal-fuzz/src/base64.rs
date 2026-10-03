//! The harness for the base64 codecs in `gunmetal_core::base64`, which
//! read pictures embedded in Vorbis comments and tokens from requests.

use gunmetal_core::base64::{self, Alphabet, B64Error};
use gunmetal_core::untrusted::Untrusted;

/// The most octets any call may decode to, small enough that fuzzing
/// reaches it often.
pub const MAX_OUT: usize = 48;

/// What the decoder reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// [`base64::decode`] in the standard alphabet.
    pub standard: Result<Vec<u8>, B64Error>,
    /// [`base64::decode`] in the URL-safe alphabet.
    pub url_safe: Result<Vec<u8>, B64Error>,
}

/// Feeds `data` to [`base64::decode`] in both alphabets, with at most
/// [`MAX_OUT`] octets out, and re-encodes what it accepts with
/// [`base64::encode`].
///
/// # Panics
///
/// Panics when a result breaks an invariant that holds for every input.
/// Decoded data must fit [`MAX_OUT`] and encode back to the input, apart
/// from padding, so each value has one written form. An error must point
/// inside the input: an invalid octet at its offset, the input's own
/// length, the offset of a character within it, or a size over the cap.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let [standard, url_safe] = [Alphabet::Standard, Alphabet::UrlSafe].map(|alphabet| {
        let decoded = base64::decode(Untrusted::new(data), alphabet, MAX_OUT);
        assert!(
            match &decoded {
                Ok(octets) =>
                    octets.len() <= MAX_OUT
                        && base64::encode(octets, alphabet)
                            .trim_end_matches('=')
                            .as_bytes()
                            == data
                                .strip_suffix(b"==")
                                .or(data.strip_suffix(b"="))
                                .unwrap_or(data),
                Err(B64Error::InvalidByte { offset, byte }) => data.get(*offset) == Some(byte),
                Err(B64Error::InvalidLength { len }) => *len == data.len(),
                Err(B64Error::TrailingBits { offset }) => *offset < data.len(),
                Err(B64Error::TooLong { needed, max }) => *max == MAX_OUT && *needed > MAX_OUT,
            },
            "{alphabet:?} decoded {data:02X?} to {decoded:?}"
        );
        decoded
    });
    Outcome { standard, url_safe }
}
