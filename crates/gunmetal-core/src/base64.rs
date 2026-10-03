//! Base64 (RFC 4648), the two small codecs the project owns instead of a
//! dependency: the standard alphabet for pictures embedded in Vorbis
//! comments, and the URL-safe alphabet for tokens.
//!
//! Decoding accepts each value in exactly one form, with or without its
//! padding: no whitespace, no characters of the other alphabet, and no
//! stray bits in the last character, so a token cannot be written two
//! ways. The output size is checked against the caller's cap before
//! anything is decoded. The text to decode arrives [`Untrusted`]: a token
//! from a request or a picture from a tag (SEC-TM-031).

use crate::untrusted::Untrusted;

/// The standard alphabet, RFC 4648 section 4.
const STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
/// The URL- and filename-safe alphabet, RFC 4648 section 5.
const URL_SAFE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Which of RFC 4648's two alphabets to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alphabet {
    /// Section 4, with `+` and `/`, written with padding.
    Standard,
    /// Section 5, with `-` and `_`, written without padding.
    UrlSafe,
}

/// Why base64 text could not be decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum B64Error {
    /// An octet outside the alphabet, including whitespace and padding
    /// anywhere but the end.
    InvalidByte {
        /// Where it is.
        offset: usize,
        /// The octet.
        byte: u8,
    },
    /// A length no encoding produces, or padding that does not complete
    /// the last group of four.
    InvalidLength {
        /// The length of the input.
        len: usize,
    },
    /// The last character carries bits that are not zero.
    TrailingBits {
        /// Where the last character is.
        offset: usize,
    },
    /// The decoded data would be longer than allowed.
    TooLong {
        /// Octets the input decodes to.
        needed: usize,
        /// Octets allowed.
        max: usize,
    },
}

/// Encodes `input`, with padding in the standard alphabet and without it
/// in the URL-safe one.
#[must_use]
pub fn encode(input: &[u8], alphabet: Alphabet) -> String {
    let symbols = match alphabet {
        Alphabet::Standard => STANDARD,
        Alphabet::UrlSafe => URL_SAFE,
    };
    let mut text = String::new();
    for group in input.chunks(3) {
        let mut octets = [0_u8; 3];
        for (slot, &octet) in octets.iter_mut().zip(group) {
            *slot = octet;
        }
        let [first, second, third] = octets;
        let bits = u32::from_be_bytes([0, first, second, third]);
        // A group of n octets needs n + 1 characters.
        for (index, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if index <= group.len() {
                let sextet = usize::try_from(bits >> shift & 0x3F).unwrap_or_default();
                let symbol = symbols.get(sextet).copied().unwrap_or_default();
                text.push(char::from(symbol));
            } else if alphabet == Alphabet::Standard {
                text.push('=');
            }
        }
    }
    text
}

/// Decodes `input` into at most `max_out` octets.
///
/// # Errors
///
/// A [`B64Error`] for input that is not base64 in `alphabet`, or that
/// decodes to more than `max_out` octets.
pub fn decode(
    input: Untrusted<&[u8]>,
    alphabet: Alphabet,
    max_out: usize,
) -> Result<Vec<u8>, B64Error> {
    let input = input.into_inner();
    let (data, padded) = match input {
        [data @ .., b'=', b'='] | [data @ .., b'='] => (data, true),
        _ => (input, false),
    };
    if (padded && input.len() % 4 != 0) || data.len() % 4 == 1 {
        return Err(B64Error::InvalidLength { len: input.len() });
    }
    let partial = match data.len() % 4 {
        2 => 1,
        3 => 2,
        _ => 0,
    };
    let needed = (data.len() / 4).saturating_mul(3).saturating_add(partial);
    if needed > max_out {
        return Err(B64Error::TooLong {
            needed,
            max: max_out,
        });
    }
    let mut decoded = Vec::new();
    let mut bits = 0_u32;
    for (offset, &byte) in data.iter().enumerate() {
        let value = sextet(byte, alphabet).ok_or(B64Error::InvalidByte { offset, byte })?;
        // The shift leaves the low six bits zero, so this never saturates.
        bits = (bits << 6).saturating_add(u32::from(value));
        let shift = match offset % 4 {
            1 => 4,
            2 => 2,
            3 => 0,
            _ => continue,
        };
        let [low, ..] = (bits >> shift).to_le_bytes();
        decoded.push(low);
    }
    let leftover = match data.len() % 4 {
        2 => 0xF,
        3 => 0x3,
        _ => 0,
    };
    if bits & leftover != 0 {
        return Err(B64Error::TrailingBits {
            offset: data.len().saturating_sub(1),
        });
    }
    Ok(decoded)
}

/// The value of one base64 character.
fn sextet(symbol: u8, alphabet: Alphabet) -> Option<u8> {
    match (symbol, alphabet) {
        (b'A'..=b'Z', _) => Some(symbol.wrapping_sub(b'A')),
        (b'a'..=b'z', _) => Some(symbol.wrapping_sub(b'a').wrapping_add(26)),
        (b'0'..=b'9', _) => Some(symbol.wrapping_sub(b'0').wrapping_add(52)),
        (b'+', Alphabet::Standard) | (b'-', Alphabet::UrlSafe) => Some(62),
        (b'/', Alphabet::Standard) | (b'_', Alphabet::UrlSafe) => Some(63),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;
    // Direct imports: Qodana does not resolve these macros through `prelude::*`.
    use proptest::{prop_oneof, proptest};

    /// Decodes `input` as it arrives from outside.
    fn decode(input: &[u8], alphabet: Alphabet, max_out: usize) -> Result<Vec<u8>, B64Error> {
        super::decode(Untrusted::new(input), alphabet, max_out)
    }

    /// RFC 4648, section 10, with the URL-safe form of each.
    const VECTORS: [(&[u8], &str, &str); 7] = [
        (b"", "", ""),
        (b"f", "Zg==", "Zg"),
        (b"fo", "Zm8=", "Zm8"),
        (b"foo", "Zm9v", "Zm9v"),
        (b"foob", "Zm9vYg==", "Zm9vYg"),
        (b"fooba", "Zm9vYmE=", "Zm9vYmE"),
        (b"foobar", "Zm9vYmFy", "Zm9vYmFy"),
    ];

    /// Octets whose encodings use the two characters the alphabets differ
    /// in, and every class of character.
    const SPECIAL: [(&[u8], &str, &str); 5] = [
        (&[0xFB, 0xFF], "+/8=", "-_8"),
        (&[0xFF, 0xFF, 0xFF], "////", "____"),
        (&[0xF8], "+A==", "-A"),
        (&[0x00, 0x00, 0x00], "AAAA", "AAAA"),
        (
            &[0x00, 0x10, 0x83, 0x10, 0x51, 0x87, 0x20, 0x92, 0x8B],
            "ABCDEFGHIJKL",
            "ABCDEFGHIJKL",
        ),
    ];

    #[test]
    fn encodes_the_rfc_4648_test_vectors() {
        for (octets, standard, url_safe) in VECTORS.into_iter().chain(SPECIAL) {
            assert_eq!(encode(octets, Alphabet::Standard), standard);
            assert_eq!(encode(octets, Alphabet::UrlSafe), url_safe);
        }
    }

    #[test]
    fn decodes_the_rfc_4648_test_vectors_with_and_without_padding() {
        for (octets, standard, url_safe) in VECTORS.into_iter().chain(SPECIAL) {
            let unpadded = standard.trim_end_matches('=');
            let url_padded = format!("{url_safe}{}", &standard[unpadded.len()..]);
            assert_eq!(
                decode(standard.as_bytes(), Alphabet::Standard, 64),
                Ok(octets.to_vec())
            );
            assert_eq!(
                decode(unpadded.as_bytes(), Alphabet::Standard, 64),
                Ok(octets.to_vec())
            );
            assert_eq!(
                decode(url_safe.as_bytes(), Alphabet::UrlSafe, 64),
                Ok(octets.to_vec())
            );
            assert_eq!(
                decode(url_padded.as_bytes(), Alphabet::UrlSafe, 64),
                Ok(octets.to_vec())
            );
        }
    }

    #[test]
    fn decodes_every_character_of_both_alphabets() {
        let standard = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let url_safe = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        // Sixty-four characters in order are the 48 octets 0x00, 0x10,
        // 0x83, ... that count up six bits at a time.
        let expected: Vec<u8> = [
            0x00, 0x10, 0x83, 0x10, 0x51, 0x87, 0x20, 0x92, 0x8B, 0x30, 0xD3, 0x8F, 0x41, 0x14,
            0x93, 0x51, 0x55, 0x97, 0x61, 0x96, 0x9B, 0x71, 0xD7, 0x9F, 0x82, 0x18, 0xA3, 0x92,
            0x59, 0xA7, 0xA2, 0x9A, 0xAB, 0xB2, 0xDB, 0xAF, 0xC3, 0x1C, 0xB3, 0xD3, 0x5D, 0xB7,
            0xE3, 0x9E, 0xBB, 0xF3, 0xDF, 0xBF,
        ]
        .to_vec();
        assert_eq!(
            decode(standard, Alphabet::Standard, 48),
            Ok(expected.clone())
        );
        assert_eq!(
            decode(url_safe, Alphabet::UrlSafe, 48),
            Ok(expected.clone())
        );
        assert_eq!(encode(&expected, Alphabet::Standard).as_bytes(), standard);
        assert_eq!(encode(&expected, Alphabet::UrlSafe).as_bytes(), url_safe);
    }

    #[test]
    fn refuses_octets_outside_the_alphabet() {
        let cases: [(&[u8], Alphabet, usize, u8); 9] = [
            (b"Zm9v YmF", Alphabet::Standard, 4, b' '),
            (b"Zm9v\nYmF", Alphabet::Standard, 4, b'\n'),
            (b"-_8=", Alphabet::Standard, 0, b'-'),
            (b"+/8=", Alphabet::UrlSafe, 0, b'+'),
            (b"Zm9v_mFy", Alphabet::Standard, 4, b'_'),
            (b"Zm9v/mFy", Alphabet::UrlSafe, 4, b'/'),
            (b"Zm=vYmFy", Alphabet::Standard, 2, b'='),
            (b"====", Alphabet::Standard, 0, b'='),
            (b"Zm9\xC3\xA9mFy", Alphabet::Standard, 3, 0xC3),
        ];
        for (input, alphabet, offset, byte) in cases {
            assert_eq!(
                decode(input, alphabet, 64),
                Err(B64Error::InvalidByte { offset, byte }),
                "{input:?}"
            );
        }
    }

    #[test]
    fn refuses_lengths_no_encoding_produces() {
        for input in [
            &b"Z"[..],
            b"Zm9vY",
            b"=",
            b"Zg=",
            b"Zm9v=",
            b"Zg===",
            b"Zm9vYmF==",
        ] {
            assert_eq!(
                decode(input, Alphabet::Standard, 64),
                Err(B64Error::InvalidLength { len: input.len() }),
                "{input:?}"
            );
        }
    }

    /// Only one encoding of each value is accepted, so a token cannot be
    /// written in two ways.
    #[test]
    fn refuses_a_last_character_with_bits_left_over() {
        let cases: [(&[u8], usize); 4] = [(b"Zh==", 1), (b"Zh", 1), (b"Zm9=", 2), (b"Zm9vYmF", 6)];
        for (input, offset) in cases {
            assert_eq!(
                decode(input, Alphabet::Standard, 64),
                Err(B64Error::TrailingBits { offset }),
                "{input:?}"
            );
        }
    }

    #[test]
    fn refuses_output_over_the_cap_before_decoding() {
        assert_eq!(
            decode(b"Zm9vYmFy", Alphabet::Standard, 5),
            Err(B64Error::TooLong { needed: 6, max: 5 })
        );
        assert_eq!(
            decode(b"Zm9vYmE", Alphabet::UrlSafe, 4),
            Err(B64Error::TooLong { needed: 5, max: 4 })
        );
        assert_eq!(
            decode(b"Zm9vYg==", Alphabet::Standard, 3),
            Err(B64Error::TooLong { needed: 4, max: 3 })
        );
        // The cap is checked first, so even an invalid octet past it is
        // never looked at.
        assert_eq!(
            decode(b"Zm9v!!!!", Alphabet::Standard, 2),
            Err(B64Error::TooLong { needed: 6, max: 2 })
        );
        assert_eq!(
            decode(b"Zm9vYmFy", Alphabet::Standard, 6),
            Ok(b"foobar".to_vec())
        );
        assert_eq!(decode(b"", Alphabet::Standard, 0), Ok(vec![]));
    }

    proptest! {
        #[test]
        fn decoding_what_was_encoded_returns_it(octets in vec(any::<u8>(), 0..64)) {
            for alphabet in [Alphabet::Standard, Alphabet::UrlSafe] {
                let text = encode(&octets, alphabet);
                prop_assert_eq!(decode(text.as_bytes(), alphabet, octets.len()), Ok(octets.clone()));
                prop_assert_eq!(
                    decode(text.trim_end_matches('=').as_bytes(), alphabet, octets.len()),
                    Ok(octets.clone())
                );
            }
        }

        #[test]
        fn decoded_output_never_exceeds_the_cap(
            input in vec(prop_oneof![Just(b'='), Just(b'A'), Just(b'/'), Just(b'-'), any::<u8>()], 0..48),
            max_out in 0_usize..40,
        ) {
            let decoded = decode(&input, Alphabet::Standard, max_out).unwrap_or_default();
            prop_assert!(decoded.len() <= max_out);
        }
    }
}
