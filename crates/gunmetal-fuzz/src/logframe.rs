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
    for item in &items {
        match item {
            Item::Record(record) => {
                assert!(u32::try_from(record.payload.len()).is_ok_and(|len| len <= MAX_PAYLOAD));
            }
            Item::Damaged { range } => {
                assert!(range.start < range.end && range.end <= data.len());
            }
        }
    }
    let tail = recover_tail(data);
    assert!(tail <= data.len());

    if let Ok(framed) = encode(data) {
        assert!(matches!(
            records(&framed).next(),
            Some(Item::Record(record)) if record.payload == data
        ));
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
        Err(error) => {
            assert_eq!(error, EncodeError::BadMonth { month });
            assert!(!(1..=12).contains(&month));
        }
    }

    Outcome {
        items,
        tail,
        header: header(data),
    }
}
