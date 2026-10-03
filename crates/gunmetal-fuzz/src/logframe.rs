//! The harness for log segment framing in [`gunmetal_core::logframe`].

use gunmetal_core::logframe::{
    EncodeError, HeaderError, Item, MAX_PAYLOAD, SegmentHeader, encode, encode_header, header,
    records, recover_tail,
};

/// What the log-segment framer reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome<'a> {
    /// Every item [`records`] yielded, in order.
    pub items: Vec<Item<'a>>,
    /// [`recover_tail`] of the same input.
    pub tail: usize,
    /// [`header`] of the same input.
    pub header: Result<SegmentHeader, HeaderError>,
}

/// Feeds `data` to [`records`], [`recover_tail`], [`header`], [`encode`]
/// and [`encode_header`].
///
/// # Panics
///
/// Panics when a yielded record's payload is longer than [`MAX_PAYLOAD`],
/// a damaged range is empty or runs past the input, [`recover_tail`]
/// returns a length past the input, [`encode`] of the input does not
/// round-trip through [`records`], or a well-formed header does not read
/// back as itself.
#[must_use]
pub fn run(data: &[u8]) -> Outcome<'_> {
    let items: Vec<Item<'_>> = records(data).collect();
    let max = usize::try_from(MAX_PAYLOAD).unwrap_or(0);
    for item in &items {
        match item {
            Item::Record(record) => {
                assert!(
                    record.payload.len() <= max,
                    "record payload {} octets exceeds the cap",
                    record.payload.len()
                );
            }
            Item::Damaged { range } => {
                assert!(
                    range.start < range.end && range.end <= data.len(),
                    "damaged range {range:?} outside {} octets",
                    data.len()
                );
            }
        }
    }
    let tail = recover_tail(data);
    assert!(
        tail <= data.len(),
        "recover_tail {tail} outside {} octets",
        data.len()
    );

    if let Ok(framed) = encode(data) {
        let got: Vec<&[u8]> = records(&framed)
            .map(|item| match item {
                Item::Record(record) => record.payload,
                Item::Damaged { range } => panic!("encoded payload damaged at {range:?}"),
            })
            .collect();
        assert_eq!(got, [data], "encode then records lost the payload");
        assert_eq!(recover_tail(&framed), framed.len());
    }

    let mut stream = [0_u8; 16];
    for (slot, byte) in stream.iter_mut().zip(data.iter().copied()) {
        *slot = byte;
    }
    let month = data.first().copied().unwrap_or(1);
    let hdr = SegmentHeader {
        stream,
        year: 2026,
        month,
    };
    match encode_header(&hdr) {
        Ok(bytes) => {
            assert_eq!(header(&bytes), Ok(hdr));
            assert_eq!(recover_tail(&bytes), bytes.len());
        }
        Err(EncodeError::BadMonth { month: got }) => {
            assert_eq!(got, month);
            assert!(
                !(1..=12).contains(&month),
                "encode_header refused month {month}"
            );
        }
        Err(EncodeError::TooLong { len, max }) => {
            panic!("a 19-octet header is under the cap: {len} > {max}");
        }
    }

    Outcome {
        items,
        tail,
        header: header(data),
    }
}
