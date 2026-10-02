//! EBML primitives (RFC 8794), the encoding underneath Matroska and `WebM`.

/// A decoded variable-size integer (RFC 8794 section 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vint {
    /// The integer with its length marker removed.
    pub value: u64,
    /// How many octets the encoding occupied.
    pub width: u8,
}

/// Why a variable-size integer could not be decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VintError {
    /// The input held no octets.
    Empty,
    /// The first octet was zero, so it carried no length marker.
    InvalidWidth,
    /// The input ended before the width declared by the first octet.
    Truncated {
        /// Octets the integer needs in total.
        needed: u8,
        /// Octets the input actually held.
        available: usize,
    },
}

/// Decodes the variable-size integer at the start of `input`.
///
/// # Errors
///
/// Returns a [`VintError`] when `input` does not start with a complete,
/// well-formed variable-size integer.
pub fn decode_vint(input: &[u8]) -> Result<Vint, VintError> {
    let Some(&first) = input.first() else {
        return Err(VintError::Empty);
    };
    if first == 0 {
        return Err(VintError::InvalidWidth);
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a u8 has at most 8 leading zeros"
    )]
    let width = first.leading_zeros() as u8 + 1;
    let Some(octets) = input.get(..usize::from(width)) else {
        return Err(VintError::Truncated {
            needed: width,
            available: input.len(),
        });
    };
    let mut padded = [0; 8];
    padded[8 - octets.len()..].copy_from_slice(octets);
    let marker = 1 << (7 * u32::from(width));
    Ok(Vint {
        value: u64::from_be_bytes(padded) ^ marker,
        width,
    })
}

/// An element ID with its length marker retained, which is how the Matroska
/// specification writes them (for example `0x1A45DFA3` for the EBML header).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ElementId(pub u32);

/// The size of an element's body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSize {
    /// The body is exactly this many octets long.
    Known(u64),
    /// The body runs until its parent ends or a sibling starts, as written by
    /// live muxers that cannot seek back to fill the size in.
    Unknown,
}

/// The ID and data size that open every EBML element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementHeader {
    /// Which element this is.
    pub id: ElementId,
    /// How long the element's body is.
    pub size: DataSize,
    /// Octets occupied by the ID and the data size together.
    pub header_len: u8,
}

/// Why an element header could not be decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderError {
    /// The element ID was malformed.
    Id(VintError),
    /// The element ID was longer than the four octets Matroska allows.
    IdTooLong {
        /// Octets the ID declared.
        width: u8,
    },
    /// The data size was malformed.
    Size(VintError),
}

/// The longest element ID Matroska allows, in octets.
const MAX_ID_WIDTH: u8 = 4;

/// Decodes the element header at the start of `input`.
///
/// # Errors
///
/// Returns a [`HeaderError`] when `input` does not start with a complete,
/// well-formed element ID followed by a data size.
pub fn decode_element_header(input: &[u8]) -> Result<ElementHeader, HeaderError> {
    let id = decode_vint(input).map_err(HeaderError::Id)?;
    if id.width > MAX_ID_WIDTH {
        return Err(HeaderError::IdTooLong { width: id.width });
    }
    let (id_octets, rest) = input.split_at(usize::from(id.width));
    let mut padded = [0; 4];
    padded[4 - id_octets.len()..].copy_from_slice(id_octets);
    let size = decode_vint(rest).map_err(HeaderError::Size)?;
    let all_data_bits_set = (1 << (7 * u32::from(size.width))) - 1;
    Ok(ElementHeader {
        id: ElementId(u32::from_be_bytes(padded)),
        size: if size.value == all_data_bits_set {
            DataSize::Unknown
        } else {
            DataSize::Known(size.value)
        },
        header_len: id.width + size.width,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// Reference encoder used only as a test oracle; it shares no code with
    /// the decoder under test.
    fn encode_vint(value: u64, width: u8) -> Vec<u8> {
        let marker = 1_u64 << (7 * u32::from(width));
        assert!(value < marker, "value does not fit in {width} octets");
        (value | marker).to_be_bytes()[8 - usize::from(width)..].to_vec()
    }

    proptest! {
        #[test]
        fn decodes_whatever_the_reference_encoder_writes(
            width in 1_u8..=8,
            raw in any::<u64>(),
            tail in vec(any::<u8>(), 0..4),
        ) {
            let value = raw % (1_u64 << (7 * u32::from(width)));
            let mut bytes = encode_vint(value, width);
            bytes.extend(tail);
            prop_assert_eq!(decode_vint(&bytes), Ok(Vint { value, width }));
        }

        #[test]
        fn reports_truncation_for_every_proper_prefix(
            width in 2_u8..=8,
            raw in any::<u64>(),
            keep in 1_usize..8,
        ) {
            let value = raw % (1_u64 << (7 * u32::from(width)));
            let bytes = encode_vint(value, width);
            let keep = keep.min(usize::from(width) - 1);
            prop_assert_eq!(
                decode_vint(&bytes[..keep]),
                Err(VintError::Truncated { needed: width, available: keep })
            );
        }

        #[test]
        fn decodes_any_header_the_reference_encoder_writes(
            id_width in 1_u8..=4,
            id_raw in any::<u64>(),
            size_width in 1_u8..=8,
            // `None` forces the all-ones "unknown size" pattern, which random
            // values almost never produce on their own.
            size_raw in prop_oneof![Just(None), any::<u64>().prop_map(Some)],
            body in vec(any::<u8>(), 0..4),
        ) {
            let id_marker = 1_u64 << (7 * u32::from(id_width));
            let id_data = id_raw % id_marker;
            let size_limit = 1_u64 << (7 * u32::from(size_width));
            let size_data = size_raw.map_or(size_limit - 1, |raw| raw % size_limit);

            let mut bytes = encode_vint(id_data, id_width);
            bytes.extend(encode_vint(size_data, size_width));
            bytes.extend(body);

            let expected_size = if size_data == size_limit - 1 {
                DataSize::Unknown
            } else {
                DataSize::Known(size_data)
            };
            prop_assert_eq!(
                decode_element_header(&bytes),
                Ok(ElementHeader {
                    id: ElementId(u32::try_from(id_data | id_marker).unwrap()),
                    size: expected_size,
                    header_len: id_width + size_width,
                })
            );
        }
    }

    #[test]
    fn decodes_one_octet_vints() {
        assert_eq!(decode_vint(&[0x81]), Ok(Vint { value: 1, width: 1 }));
        assert_eq!(decode_vint(&[0x80]), Ok(Vint { value: 0, width: 1 }));
        assert_eq!(
            decode_vint(&[0xFF]),
            Ok(Vint {
                value: 127,
                width: 1
            })
        );
    }

    #[test]
    fn decodes_every_width_from_two_to_eight_octets() {
        let cases: [(&[u8], u64, u8); 9] = [
            (&[0x40, 0x01], 1, 2),
            (&[0x7F, 0xFF], 16_383, 2),
            (&[0x20, 0x00, 0x01], 1, 3),
            (&[0x10, 0x00, 0x00, 0x01], 1, 4),
            (&[0x08, 0x00, 0x00, 0x00, 0x01], 1, 5),
            (&[0x04, 0x00, 0x00, 0x00, 0x00, 0x01], 1, 6),
            (&[0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01], 1, 7),
            (&[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02], 2, 8),
            (
                &[0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
                72_057_594_037_927_935,
                8,
            ),
        ];
        for (bytes, value, width) in cases {
            assert_eq!(
                decode_vint(bytes),
                Ok(Vint { value, width }),
                "bytes {bytes:02X?}"
            );
        }
    }

    #[test]
    fn keeps_every_data_bit_of_a_multi_octet_vint() {
        assert_eq!(
            decode_vint(&[0x12, 0x34, 0x56, 0x78]),
            Ok(Vint {
                value: 0x0234_5678,
                width: 4
            })
        );
    }

    #[test]
    fn rejects_empty_input() {
        assert_eq!(decode_vint(&[]), Err(VintError::Empty));
    }

    #[test]
    fn rejects_a_first_octet_with_no_length_marker() {
        assert_eq!(decode_vint(&[0x00]), Err(VintError::InvalidWidth));
        assert_eq!(
            decode_vint(&[0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]),
            Err(VintError::InvalidWidth)
        );
    }

    #[test]
    fn rejects_input_shorter_than_the_declared_width() {
        assert_eq!(
            decode_vint(&[0x40]),
            Err(VintError::Truncated {
                needed: 2,
                available: 1
            })
        );
        assert_eq!(
            decode_vint(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
            Err(VintError::Truncated {
                needed: 8,
                available: 7
            })
        );
    }

    #[test]
    fn decodes_headers_with_ids_of_every_legal_width() {
        // Void, EBMLVersion, TimestampScale and the EBML header itself.
        let cases: [(&[u8], u32, u64, u8); 4] = [
            (&[0xEC, 0x80], 0xEC, 0, 2),
            (&[0x42, 0x86, 0x81], 0x4286, 1, 3),
            (&[0x2A, 0xD7, 0xB1, 0x83], 0x002A_D7B1, 3, 4),
            (&[0x1A, 0x45, 0xDF, 0xA3, 0x9F], 0x1A45_DFA3, 31, 5),
        ];
        for (bytes, id, size, header_len) in cases {
            assert_eq!(
                decode_element_header(bytes),
                Ok(ElementHeader {
                    id: ElementId(id),
                    size: DataSize::Known(size),
                    header_len,
                }),
                "bytes {bytes:02X?}"
            );
        }
    }

    #[test]
    fn decodes_multi_octet_data_sizes() {
        assert_eq!(
            decode_element_header(&[0x1F, 0x43, 0xB6, 0x75, 0x10, 0x20, 0x30, 0x40]),
            Ok(ElementHeader {
                id: ElementId(0x1F43_B675),
                size: DataSize::Known(0x0020_3040),
                header_len: 8,
            })
        );
        assert_eq!(
            decode_element_header(&[0xEC, 0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE]),
            Ok(ElementHeader {
                id: ElementId(0xEC),
                size: DataSize::Known(72_057_594_037_927_934),
                header_len: 9,
            })
        );
    }

    #[test]
    fn ignores_the_element_body_after_the_header() {
        assert_eq!(
            decode_element_header(&[0x42, 0x86, 0x81, 0x01, 0xFF, 0xFF]),
            Ok(ElementHeader {
                id: ElementId(0x4286),
                size: DataSize::Known(1),
                header_len: 3,
            })
        );
    }

    #[test]
    fn reports_an_unknown_size_when_every_data_bit_is_set() {
        // The last case is a Segment written by a live muxer.
        let cases: [(&[u8], u32, u8); 3] = [
            (&[0xEC, 0xFF], 0xEC, 2),
            (&[0xEC, 0x7F, 0xFF], 0xEC, 3),
            (
                &[
                    0x18, 0x53, 0x80, 0x67, 0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                ],
                0x1853_8067,
                12,
            ),
        ];
        for (bytes, id, header_len) in cases {
            assert_eq!(
                decode_element_header(bytes),
                Ok(ElementHeader {
                    id: ElementId(id),
                    size: DataSize::Unknown,
                    header_len,
                }),
                "bytes {bytes:02X?}"
            );
        }
    }

    #[test]
    fn treats_sizes_next_to_the_unknown_marker_as_known() {
        let cases: [(&[u8], u64, u8); 3] = [
            (&[0xEC, 0xFE], 126, 2),
            (&[0xEC, 0x40, 0x7F], 127, 3),
            (&[0xEC, 0x7F, 0xFE], 16_382, 3),
        ];
        for (bytes, size, header_len) in cases {
            assert_eq!(
                decode_element_header(bytes),
                Ok(ElementHeader {
                    id: ElementId(0xEC),
                    size: DataSize::Known(size),
                    header_len,
                }),
                "bytes {bytes:02X?}"
            );
        }
    }

    #[test]
    fn rejects_ids_longer_than_four_octets() {
        assert_eq!(
            decode_element_header(&[0x08, 0x00, 0x00, 0x00, 0x01, 0x81]),
            Err(HeaderError::IdTooLong { width: 5 })
        );
        assert_eq!(
            decode_element_header(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x81]),
            Err(HeaderError::IdTooLong { width: 8 })
        );
    }

    #[test]
    fn reports_which_half_of_the_header_was_malformed() {
        let cases: [(&[u8], HeaderError); 6] = [
            (&[], HeaderError::Id(VintError::Empty)),
            (&[0x00, 0x81], HeaderError::Id(VintError::InvalidWidth)),
            (
                &[0x1A, 0x45],
                HeaderError::Id(VintError::Truncated {
                    needed: 4,
                    available: 2,
                }),
            ),
            (&[0xEC], HeaderError::Size(VintError::Empty)),
            (&[0xEC, 0x00], HeaderError::Size(VintError::InvalidWidth)),
            (
                &[0x1A, 0x45, 0xDF, 0xA3, 0x40],
                HeaderError::Size(VintError::Truncated {
                    needed: 2,
                    available: 1,
                }),
            ),
        ];
        for (bytes, error) in cases {
            assert_eq!(
                decode_element_header(bytes),
                Err(error),
                "bytes {bytes:02X?}"
            );
        }
    }

    #[test]
    fn ignores_bytes_after_the_vint() {
        assert_eq!(
            decode_vint(&[0x81, 0xFF, 0xFF]),
            Ok(Vint { value: 1, width: 1 })
        );
        assert_eq!(
            decode_vint(&[0x40, 0x02, 0xFF]),
            Ok(Vint { value: 2, width: 2 })
        );
    }
}
