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
    split_vint(input).map(|split| split.vint)
}

/// A variable-size integer taken from the start of its input.
struct SplitVint<'a> {
    /// The integer.
    vint: Vint,
    /// The octets that encoded it, length marker included.
    octets: &'a [u8],
    /// The input after those octets.
    rest: &'a [u8],
}

/// Decodes the variable-size integer at the start of `input` and splits it
/// from the octets after it.
fn split_vint(input: &[u8]) -> Result<SplitVint<'_>, VintError> {
    let Some(&first) = input.first() else {
        return Err(VintError::Empty);
    };
    if first == 0 {
        return Err(VintError::InvalidWidth);
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::arithmetic_side_effects,
        reason = "first is non-zero, so it has at most 7 leading zeros and the width is 1 to 8"
    )]
    let width = first.leading_zeros() as u8 + 1;
    let Some((octets, rest)) = input.split_at_checked(usize::from(width)) else {
        return Err(VintError::Truncated {
            needed: width,
            available: input.len(),
        });
    };
    let marker = 1 << (7 * u32::from(width));
    Ok(SplitVint {
        vint: Vint {
            value: u64::from_be_bytes(right_aligned(octets)) ^ marker,
            width,
        },
        octets,
        rest,
    })
}

/// `octets`, of which there are at most `N`, right-aligned in `N` zero
/// octets, so that reading the result big-endian gives the integer they
/// encode.
fn right_aligned<const N: usize>(octets: &[u8]) -> [u8; N] {
    let mut padded = [0; N];
    for (slot, &octet) in padded.iter_mut().rev().zip(octets.iter().rev()) {
        *slot = octet;
    }
    padded
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
    split_element_header(input).map(|(header, _)| header)
}

/// Decodes the element header at the start of `input` and splits it from the
/// octets after it.
fn split_element_header(input: &[u8]) -> Result<(ElementHeader, &[u8]), HeaderError> {
    let id = split_vint(input).map_err(HeaderError::Id)?;
    if id.vint.width > MAX_ID_WIDTH {
        return Err(HeaderError::IdTooLong {
            width: id.vint.width,
        });
    }
    let size = split_vint(id.rest).map_err(HeaderError::Size)?;
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the width is 1 to 8, so the shift is at most 56 and its result at least 128"
    )]
    let all_data_bits_set = (1 << (7 * u32::from(size.vint.width))) - 1;
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "an ID is at most 4 octets and a data size at most 8, so the sum is at most 12"
    )]
    let header_len = id.vint.width + size.vint.width;
    let header = ElementHeader {
        id: ElementId(u32::from_be_bytes(right_aligned(id.octets))),
        size: if size.vint.value == all_data_bits_set {
            DataSize::Unknown
        } else {
            DataSize::Known(size.vint.value)
        },
        header_len,
    };
    Ok((header, size.rest))
}

/// An element whose body lies entirely inside the parsed input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Element<'a> {
    /// Which element this is.
    pub id: ElementId,
    /// The element's body, borrowed from the input.
    pub body: &'a [u8],
}

/// Why iteration over sibling elements stopped early.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementError {
    /// An element header was malformed.
    Header {
        /// Where the header started, in octets from the start of the input.
        offset: usize,
        /// What was wrong with it.
        error: HeaderError,
    },
    /// An element's body ran past the end of the input.
    BodyTruncated {
        /// Where the element started, in octets from the start of the input.
        offset: usize,
        /// Which element it was.
        id: ElementId,
        /// Octets the body declared.
        needed: u64,
        /// Octets left in the input after the header.
        available: usize,
    },
    /// An element declared an unknown size, so its end cannot be found
    /// without interpreting its children.
    UnknownSize {
        /// Where the element started, in octets from the start of the input.
        offset: usize,
        /// Which element it was.
        id: ElementId,
    },
}

impl ElementError {
    /// Where the offending element started, in octets from the start of the
    /// input.
    #[must_use]
    pub const fn offset(&self) -> usize {
        match *self {
            Self::Header { offset, .. }
            | Self::BodyTruncated { offset, .. }
            | Self::UnknownSize { offset, .. } => offset,
        }
    }
}

/// Iterator over the sibling elements in a buffer. See [`elements`].
#[derive(Debug, Clone)]
pub struct Elements<'a> {
    input: &'a [u8],
    /// The part of `input` not read yet, which is always a suffix of it.
    /// Empty once finished.
    rest: &'a [u8],
}

impl<'a> Iterator for Elements<'a> {
    type Item = Result<Element<'a>, ElementError>;

    fn next(&mut self) -> Option<Self::Item> {
        let rest = self.rest;
        if rest.is_empty() {
            return None;
        }
        // `rest` is a suffix of `input`, so the subtraction never saturates.
        let offset = self.input.len().saturating_sub(rest.len());
        // Finished unless this element turns out to be well formed.
        self.rest = &[];

        let (header, after_header) = match split_element_header(rest) {
            Ok(split) => split,
            Err(error) => return Some(Err(ElementError::Header { offset, error })),
        };
        let DataSize::Known(needed) = header.size else {
            return Some(Err(ElementError::UnknownSize {
                offset,
                id: header.id,
            }));
        };
        let Some((body, after_body)) = usize::try_from(needed)
            .ok()
            .and_then(|len| after_header.split_at_checked(len))
        else {
            return Some(Err(ElementError::BodyTruncated {
                offset,
                id: header.id,
                needed,
                available: after_header.len(),
            }));
        };
        self.rest = after_body;
        Some(Ok(Element {
            id: header.id,
            body,
        }))
    }
}

/// Iterates over the sibling elements packed back to back in `input`.
///
/// After yielding an error the iterator is finished and yields nothing more.
#[must_use]
pub fn elements(input: &[u8]) -> Elements<'_> {
    Elements { input, rest: input }
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the oracles and generators work with widths of at most 8 and inputs of a few hundred octets"
)]
mod tests {
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;
    // Qodana does not expand `proptest!` or resolve `prop_oneof!` through `prelude::*`.
    use proptest::prop_oneof;
    use proptest::test_runner::{Config, TestRunner};

    /// Reference encoder used only as a test oracle; it shares no code with
    /// the decoder under test.
    fn encode_vint(value: u64, width: u8) -> Vec<u8> {
        let marker = 1_u64 << (7 * u32::from(width));
        assert!(value < marker, "value does not fit in {width} octets");
        (value | marker).to_be_bytes()[8 - usize::from(width)..].to_vec()
    }

    /// Collects every item from [`elements`], failing instead of hanging if
    /// the iterator yields more than the input could hold. An element is at
    /// least two octets, so `len / 2` elements plus one error is the ceiling.
    fn collect_all(input: &[u8]) -> Vec<Result<Element<'_>, ElementError>> {
        let octets = input.len();
        let ceiling = octets / 2 + 1;
        let items: Vec<_> = elements(input).take(ceiling + 1).collect();
        let yielded = items.len();
        assert!(
            yielded <= ceiling,
            "iterator yielded {yielded} items from {octets} octets"
        );
        items
    }

    /// An element as the tests describe it: raw ID with marker, and body.
    type Described = (u32, Vec<u8>);

    /// Generates an element description together with its encoding.
    fn encoded_element() -> impl Strategy<Value = (Described, Vec<u8>)> {
        (1_u8..=4, any::<u64>(), 1_u8..=8, vec(any::<u8>(), 0..64)).prop_map(
            |(id_width, id_raw, size_width, body)| {
                let id_marker = 1_u64 << (7 * u32::from(id_width));
                let id_data = id_raw % id_marker;
                let mut bytes = encode_vint(id_data, id_width);
                bytes.extend(encode_vint(u64::try_from(body.len()).unwrap(), size_width));
                bytes.extend(&body);
                ((u32::try_from(id_data | id_marker).unwrap(), body), bytes)
            },
        )
    }

    #[test]
    fn reports_where_each_kind_of_error_happened() {
        assert_eq!(
            ElementError::Header {
                offset: 7,
                error: HeaderError::Id(VintError::Empty),
            }
            .offset(),
            7
        );
        assert_eq!(
            ElementError::BodyTruncated {
                offset: 11,
                id: ElementId(0xEC),
                needed: 3,
                available: 1,
            }
            .offset(),
            11
        );
        assert_eq!(
            ElementError::UnknownSize {
                offset: 13,
                id: ElementId(0xEC),
            }
            .offset(),
            13
        );
    }

    #[test]
    fn yields_every_element_the_reference_encoder_writes() {
        TestRunner::new(Config::default())
            .run(&vec(encoded_element(), 0..6), |encoded| {
                let bytes: Vec<u8> = encoded
                    .iter()
                    .flat_map(|(_, bytes)| bytes.clone())
                    .collect();
                let expected: Vec<_> = encoded
                    .iter()
                    .map(|((id, body), _)| {
                        Ok(Element {
                            id: ElementId(*id),
                            body,
                        })
                    })
                    .collect();
                prop_assert_eq!(collect_all(&bytes), expected);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn yields_whole_elements_then_one_error_at_any_cut() {
        TestRunner::new(Config::default())
            .run(
                &(vec(encoded_element(), 1..6), any::<usize>()),
                |(encoded, cut_seed)| {
                    let bytes: Vec<u8> = encoded
                        .iter()
                        .flat_map(|(_, bytes)| bytes.clone())
                        .collect();
                    let cut = cut_seed % bytes.len();

                    // Work out, independently of the iterator, which elements fit.
                    let mut expected = Vec::new();
                    let mut start = 0;
                    let mut cut_inside_element_at = None;
                    for ((id, body), element_bytes) in &encoded {
                        let end = start + element_bytes.len();
                        if end <= cut {
                            expected.push(Element {
                                id: ElementId(*id),
                                body,
                            });
                        } else if start < cut {
                            cut_inside_element_at = Some(start);
                        }
                        start = end;
                    }

                    let mut results = collect_all(&bytes[..cut]);
                    if let Some(error_offset) = cut_inside_element_at {
                        let last = results.pop().expect("an error for the cut element");
                        prop_assert_eq!(
                            last.as_ref().map_err(ElementError::offset),
                            Err(error_offset)
                        );
                    }
                    let expected: Vec<Result<_, ElementError>> =
                        expected.into_iter().map(Ok).collect();
                    prop_assert_eq!(results, expected);
                    Ok(())
                },
            )
            .unwrap();
    }

    #[test]
    fn decodes_whatever_the_reference_encoder_writes() {
        TestRunner::new(Config::default())
            .run(
                &(1_u8..=8, any::<u64>(), vec(any::<u8>(), 0..4)),
                |(width, raw, tail)| {
                    let value = raw % (1_u64 << (7 * u32::from(width)));
                    let mut bytes = encode_vint(value, width);
                    bytes.extend(tail);
                    prop_assert_eq!(decode_vint(&bytes), Ok(Vint { value, width }));
                    Ok(())
                },
            )
            .unwrap();
    }

    #[test]
    fn reports_truncation_for_every_proper_prefix() {
        TestRunner::new(Config::default())
            .run(
                &(2_u8..=8, any::<u64>(), 1_usize..8),
                |(width, raw, keep)| {
                    let value = raw % (1_u64 << (7 * u32::from(width)));
                    let bytes = encode_vint(value, width);
                    let keep = keep.min(usize::from(width) - 1);
                    prop_assert_eq!(
                        decode_vint(&bytes[..keep]),
                        Err(VintError::Truncated {
                            needed: width,
                            available: keep
                        })
                    );
                    Ok(())
                },
            )
            .unwrap();
    }

    #[test]
    fn decodes_any_header_the_reference_encoder_writes() {
        // `None` forces the all-ones "unknown size" pattern, which random
        // values almost never produce on their own.
        TestRunner::new(Config::default())
            .run(
                &(
                    1_u8..=4,
                    any::<u64>(),
                    1_u8..=8,
                    prop_oneof![Just(None), any::<u64>().prop_map(Some)],
                    vec(any::<u8>(), 0..4),
                ),
                |(id_width, id_raw, size_width, size_raw, body)| {
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
                    Ok(())
                },
            )
            .unwrap();
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

    /// The children of a real EBML header: `EBMLVersion` 1, `EBMLReadVersion`
    /// 1, an empty Void, and `DocType` "matroska".
    const EBML_HEADER_CHILDREN: [u8; 21] = [
        0x42, 0x86, 0x81, 0x01, // EBMLVersion
        0x42, 0xF7, 0x81, 0x01, // EBMLReadVersion
        0xEC, 0x80, // Void
        0x42, 0x82, 0x88, b'm', b'a', b't', b'r', b'o', b's', b'k', b'a', // DocType
    ];

    #[test]
    fn yields_nothing_for_empty_input() {
        assert_eq!(collect_all(&[]), vec![]);
    }

    #[test]
    fn yields_each_sibling_element_with_its_exact_body() {
        assert_eq!(
            collect_all(&EBML_HEADER_CHILDREN),
            vec![
                Ok(Element {
                    id: ElementId(0x4286),
                    body: &[0x01],
                }),
                Ok(Element {
                    id: ElementId(0x42F7),
                    body: &[0x01],
                }),
                Ok(Element {
                    id: ElementId(0xEC),
                    body: &[],
                }),
                Ok(Element {
                    id: ElementId(0x4282),
                    body: b"matroska",
                }),
            ]
        );
    }

    #[test]
    fn stops_with_an_error_when_a_body_runs_past_the_input() {
        // Everything up to and including "ma" of the DocType body.
        let cut = &EBML_HEADER_CHILDREN[..15];
        assert_eq!(
            collect_all(cut),
            vec![
                Ok(Element {
                    id: ElementId(0x4286),
                    body: &[0x01],
                }),
                Ok(Element {
                    id: ElementId(0x42F7),
                    body: &[0x01],
                }),
                Ok(Element {
                    id: ElementId(0xEC),
                    body: &[],
                }),
                Err(ElementError::BodyTruncated {
                    offset: 10,
                    id: ElementId(0x4282),
                    needed: 8,
                    available: 2,
                }),
            ]
        );
    }

    #[test]
    fn stops_with_an_error_when_a_body_is_larger_than_any_input() {
        assert_eq!(
            collect_all(&[0xEC, 0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE, 0xAA]),
            vec![Err(ElementError::BodyTruncated {
                offset: 0,
                id: ElementId(0xEC),
                needed: 72_057_594_037_927_934,
                available: 1,
            })]
        );
    }

    #[test]
    fn stops_with_the_offset_of_a_malformed_header() {
        // One good element, then an ID that declares four octets but has two.
        assert_eq!(
            collect_all(&[0x42, 0x86, 0x81, 0x01, 0x1A, 0x45]),
            vec![
                Ok(Element {
                    id: ElementId(0x4286),
                    body: &[0x01],
                }),
                Err(ElementError::Header {
                    offset: 4,
                    error: HeaderError::Id(VintError::Truncated {
                        needed: 4,
                        available: 2,
                    }),
                }),
            ]
        );
    }

    #[test]
    fn stops_with_an_error_at_an_element_of_unknown_size() {
        assert_eq!(
            collect_all(&[0xEC, 0x80, 0x1F, 0x43, 0xB6, 0x75, 0xFF, 0xAA, 0xBB]),
            vec![
                Ok(Element {
                    id: ElementId(0xEC),
                    body: &[],
                }),
                Err(ElementError::UnknownSize {
                    offset: 2,
                    id: ElementId(0x1F43_B675),
                }),
            ]
        );
    }

    #[test]
    fn keeps_returning_none_after_the_end_or_an_error() {
        let mut finished = elements(&[0xEC, 0x80]);
        assert_eq!(
            finished.next(),
            Some(Ok(Element {
                id: ElementId(0xEC),
                body: &[],
            }))
        );
        assert_eq!(finished.next(), None);
        assert_eq!(finished.next(), None);

        let mut failed = elements(&[0x00, 0xEC, 0x80]);
        assert_eq!(
            failed.next(),
            Some(Err(ElementError::Header {
                offset: 0,
                error: HeaderError::Id(VintError::InvalidWidth),
            }))
        );
        assert_eq!(failed.next(), None);
        assert_eq!(failed.next(), None);
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
