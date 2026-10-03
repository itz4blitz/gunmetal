//! The harness for log segment framing in [`gunmetal_core::logframe`].

use gunmetal_core::logframe::{
    EncodeError, FIXED_STEPS, HeaderError, Item, MAX_PAYLOAD, STEPS_PER_OCTET, SegmentHeader,
    encode, encode_header, header, records, recover_tail,
};
use gunmetal_core::parse::{Budget, ParseFault};

/// What the log-segment framer reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome<'a> {
    /// Every item [`records`] yielded, in order, or the budget fault that
    /// stopped the walk.
    pub items: Result<Vec<Item<'a>>, ParseFault>,
    /// [`recover_tail`] of the same input.
    pub tail: Result<usize, ParseFault>,
    /// [`header`] of the same input.
    pub header: Result<SegmentHeader, HeaderError>,
}

/// A documented step budget for `data`.
fn input_budget(data: &[u8]) -> Budget {
    Budget::for_input(
        u64::try_from(data.len()).unwrap_or(u64::MAX),
        STEPS_PER_OCTET,
        FIXED_STEPS,
    )
}

/// Feeds `data` to [`records`], [`recover_tail`], [`header`], [`encode`]
/// and [`encode_header`].
///
/// # Panics
///
/// Panics when a yielded record's payload is longer than [`MAX_PAYLOAD`],
/// a damaged range is empty or runs past the input, yielded items do not
/// tile the input, [`recover_tail`] disagrees with the first damaged
/// range, [`encode`] of the input does not round-trip through [`records`],
/// or a well-formed header does not read back as itself.
#[must_use]
pub fn run(data: &[u8]) -> Outcome<'_> {
    let mut records_budget = input_budget(data);
    let items: Result<Vec<Item<'_>>, ParseFault> = records(data, &mut records_budget).collect();
    let mut tail_budget = input_budget(data);
    let tail = recover_tail(data, &mut tail_budget)
        .expect("recover_tail of n octets finishes inside STEPS_PER_OCTET * n + FIXED_STEPS");
    assert!(tail <= data.len());
    match &items {
        Ok(items) => {
            let mut pos = 0_usize;
            for item in items {
                match item {
                    Item::Record(record) => {
                        assert!(
                            u32::try_from(record.payload.len()).is_ok_and(|len| len <= MAX_PAYLOAD)
                        );
                        pos = pos.saturating_add(9).saturating_add(record.payload.len());
                        assert!(pos <= data.len());
                    }
                    Item::Damaged { range } => {
                        assert!(range.start < range.end && range.end <= data.len());
                        assert_eq!(range.start, pos);
                        pos = range.end;
                    }
                }
            }
            assert_eq!(pos, data.len());
            let first_damage = items.iter().find_map(|item| match item {
                Item::Damaged { range } => Some(range.start),
                Item::Record(_) => None,
            });
            assert_eq!(
                tail,
                first_damage.unwrap_or(data.len()),
                "recover_tail must be the first damaged start, or the input length"
            );
        }
        Err(fault) => {
            let len = u64::try_from(data.len()).unwrap_or(u64::MAX);
            assert!(
                matches!(fault, ParseFault::BudgetExceeded { offset } if *offset <= len),
                "{fault:?}"
            );
        }
    }

    if let Ok(framed) = encode(data) {
        let mut framed_budget = input_budget(&framed);
        assert!(matches!(
            records(&framed, &mut framed_budget).next(),
            Some(Ok(Item::Record(record))) if record.payload == data
        ));
        let mut framed_tail = input_budget(&framed);
        assert_eq!(recover_tail(&framed, &mut framed_tail), Ok(framed.len()));
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
            let mut header_budget = input_budget(&bytes);
            assert_eq!(header(&bytes, &mut header_budget), Ok(hdr));
            let mut header_tail = input_budget(&bytes);
            assert_eq!(recover_tail(&bytes, &mut header_tail), Ok(bytes.len()));
        }
        Err(error) => {
            assert_eq!(error, EncodeError::BadMonth { month });
            assert!(!(1..=12).contains(&month));
        }
    }

    let mut header_budget = input_budget(data);
    Outcome {
        items,
        tail: Ok(tail),
        header: header(data, &mut header_budget),
    }
}
