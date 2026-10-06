//! Reading one segment back (ADR 3, section 4).
//!
//! What comes back from storage is untrusted (boundary TB10): a damaged
//! disk or a restored backup can hold anything. A scan therefore keeps only
//! what is a whole record, of this stream, numbered after the record before
//! it, and reports everything else as [`Damage`] with its byte range.
//! Damage is never repaired in place. The one exception is a torn tail, a
//! record cut off at the end of a stream's newest segment, which was never
//! acknowledged; the scan says where it starts and the log cuts it.
//!
//! A record of another stream is damage, never an event of this one, so a
//! reader of one person's stream cannot be handed another person's event by
//! whatever put it in the file (SEC-TM-024).

use std::ops::Range;

use gunmetal_core::logframe::{self, Item, RECORD_VERSION};
use gunmetal_core::parse::Budget;
use gunmetal_core::userdata::event::Stream;
use gunmetal_fs::path::LogMonth;

use crate::userlog::error::LogError;
use crate::userlog::place;
use crate::userlog::record::{self, Stamped};

/// The octets of a record around its payload: the length, the checksum and
/// the format version.
const FRAME: usize = 9;

/// Steps a scan may spend for each octet of a segment (SEC-MED-007).
///
/// A whole segment costs at most one step an octet. The rest is for
/// finding the next whole record after damage, which tries every offset of
/// the damaged range. A segment that needs more is not read to its end, and
/// the part not read is reported as damage.
pub(crate) const STEPS_PER_OCTET: u64 = 8;

/// A part of a segment that is not a record of its stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Damage {
    /// The stream the segment belongs to.
    pub stream: Stream,
    /// The segment's month.
    pub month: LogMonth,
    /// Where the part is in the segment, in octets.
    pub range: Range<usize>,
    /// The sequence number of the last record of the stream before it, if
    /// there is one. The numbers lost are after this one and before the
    /// next record the stream holds.
    pub after: Option<u64>,
    /// What is wrong with it.
    pub problem: Problem,
}

/// What is wrong with a damaged part of a segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// Octets that are not a whole record: a length that cannot be right, a
    /// checksum that does not match, or a part the scan had no steps left
    /// to read.
    Unreadable,
    /// The segment's first record is not the header of its stream and
    /// month.
    Header,
    /// A whole record that does not hold a sequence number and one event.
    NotAnEvent,
    /// A whole record that holds an event of another stream.
    Foreign,
    /// A whole record whose sequence number is not after the one before
    /// it.
    OutOfOrder,
}

/// A record of the stream and where it is in its segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Held {
    /// The octets of the whole record.
    pub(crate) range: Range<usize>,
    /// The numbered event it holds.
    pub(crate) stamped: Stamped,
}

/// What a scan of one segment found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Scan {
    /// The records of the stream, in order.
    pub(crate) records: Vec<Held>,
    /// Everything else, in order.
    pub(crate) damage: Vec<Damage>,
    /// The last entry of `damage` again, when it is octets that are not a
    /// whole record and reach the end of the segment: a torn tail, if the
    /// segment is its stream's newest.
    pub(crate) torn: Option<Range<usize>>,
}

/// The steps a scan of `bytes` may spend.
pub(crate) fn budget(bytes: &[u8]) -> Budget {
    let len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    Budget::for_input(len, STEPS_PER_OCTET, 0)
}

/// Reads `bytes`, the segment of `stream` for `month`. `last` is the
/// sequence number of the stream's last record in an earlier segment.
///
/// # Errors
///
/// Returns [`LogError::NewerRecord`] for a whole record in a format this
/// version does not write.
pub(crate) fn scan(
    stream: Stream,
    month: LogMonth,
    bytes: &[u8],
    last: Option<u64>,
    budget: &mut Budget,
) -> Result<Scan, LogError> {
    let header = place::header(stream, month);
    let mut found = Scan::default();
    let mut last = last;
    let mut pos = 0_usize;
    for item in logframe::records(bytes, budget) {
        let (range, verdict) = match item {
            Ok(Item::Record(record)) => {
                if record.version != RECORD_VERSION {
                    return Err(LogError::NewerRecord {
                        stream,
                        month,
                        offset: pos,
                        version: record.version,
                    });
                }
                let end = pos
                    .saturating_add(FRAME)
                    .saturating_add(record.payload.len());
                let verdict = judge(stream, header.get(FRAME..), pos, record.payload, last);
                (pos..end, verdict)
            }
            Ok(Item::Damaged { range }) => {
                found.torn = Some(range.clone()).filter(|tail| tail.end == bytes.len());
                (range, Err(Problem::Unreadable))
            }
            // The steps are spent: the rest of the segment was not read.
            Err(_) => (pos..bytes.len(), Err(Problem::Unreadable)),
        };
        pos = range.end;
        match verdict {
            Ok(Some(stamped)) => {
                last = Some(stamped.seq);
                found.records.push(Held { range, stamped });
            }
            Ok(None) => {}
            Err(problem) => found.damage.push(Damage {
                stream,
                month,
                range,
                after: last,
                problem,
            }),
        }
    }
    Ok(found)
}

/// What the whole record at `pos` is: the segment's header (`None`), the
/// next event of `stream`, or damage.
fn judge(
    stream: Stream,
    header: Option<&[u8]>,
    pos: usize,
    payload: &[u8],
    last: Option<u64>,
) -> Result<Option<Stamped>, Problem> {
    if pos == 0 {
        return if header == Some(payload) {
            Ok(None)
        } else {
            Err(Problem::Header)
        };
    }
    match record::read(payload) {
        None => Err(Problem::NotAnEvent),
        Some(stamped) if stamped.event.stream != stream => Err(Problem::Foreign),
        Some(stamped) if last.is_some_and(|last| stamped.seq <= last) => Err(Problem::OutOfOrder),
        Some(stamped) => Ok(Some(stamped)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::userlog::testing::{ALICE, BOB, frame, frame_version, header, of, play, record};

    /// A scan of `bytes` as Alice's segment for October 2026.
    fn scanned(bytes: &[u8], last: Option<u64>) -> Result<Scan, LogError> {
        scan(ALICE, of(2026, 10), bytes, last, &mut budget(bytes))
    }

    /// The header of Alice's segment for October 2026.
    fn october() -> Vec<u8> {
        header([0xA1; 16], 2026, 10)
    }

    fn held(range: Range<usize>, seq: u64, id: u8) -> Held {
        Held {
            range,
            stamped: Stamped {
                seq,
                event: play(id, ALICE, 1_000),
            },
        }
    }

    fn damage(range: Range<usize>, after: Option<u64>, problem: Problem) -> Damage {
        Damage {
            stream: ALICE,
            month: of(2026, 10),
            range,
            after,
            problem,
        }
    }

    #[test]
    fn reads_the_records_of_a_whole_segment_in_order() {
        // A record is nine octets around its payload.
        assert_eq!(frame(b"").len(), 9);
        let first = record(1, &play(1, ALICE, 1_000));
        let second = record(2, &play(2, ALICE, 1_000));
        let start = october().len();
        let middle = start + first.len();
        let bytes = [october(), first, second].concat();
        assert_eq!(
            scanned(&bytes, None),
            Ok(Scan {
                records: vec![held(start..middle, 1, 1), held(middle..bytes.len(), 2, 2)],
                damage: vec![],
                torn: None,
            })
        );
        assert_eq!(scanned(&[], None), Ok(Scan::default()));
        assert_eq!(scanned(&october(), Some(9)), Ok(Scan::default()));
    }

    #[test]
    fn a_record_cut_off_at_the_end_is_a_torn_tail() {
        let first = record(1, &play(1, ALICE, 1_000));
        let second = record(2, &play(2, ALICE, 1_000));
        let start = october().len();
        let middle = start + first.len();
        let bytes = [october(), first, second].concat();
        for end in middle + 1..bytes.len() {
            assert_eq!(
                scanned(&bytes[..end], None),
                Ok(Scan {
                    records: vec![held(start..middle, 1, 1)],
                    damage: vec![damage(middle..end, Some(1), Problem::Unreadable)],
                    torn: Some(middle..end),
                }),
                "cut at {end}"
            );
        }
        // A header cut short is a torn tail from the segment's start.
        assert_eq!(
            scanned(&bytes[..5], None),
            Ok(Scan {
                records: vec![],
                damage: vec![damage(0..5, None, Problem::Unreadable)],
                torn: Some(0..5),
            })
        );
    }

    #[test]
    fn damage_before_the_end_is_reported_and_the_rest_is_read() {
        let first = record(1, &play(1, ALICE, 1_000));
        let second = record(2, &play(2, ALICE, 1_000));
        let third = record(3, &play(3, ALICE, 1_000));
        let start = october().len();
        let middle = start + first.len();
        let last = middle + second.len();
        let mut bytes = [october(), first, second, third].concat();
        bytes[middle + 12] ^= 0x40;
        assert_eq!(
            scanned(&bytes, None),
            Ok(Scan {
                records: vec![held(start..middle, 1, 1), held(last..bytes.len(), 3, 3)],
                damage: vec![damage(middle..last, Some(1), Problem::Unreadable)],
                torn: None,
            })
        );
    }

    #[test]
    fn a_segment_starts_with_the_header_of_its_stream_and_month() {
        let first = record(1, &play(1, ALICE, 1_000));
        // The header of another month, of another stream, and no header.
        for wrong in [
            header([0xA1; 16], 2026, 11),
            header([0xB2; 16], 2026, 10),
            first.clone(),
        ] {
            let start = wrong.len();
            let bytes = [wrong, first.clone()].concat();
            assert_eq!(
                scanned(&bytes, None),
                Ok(Scan {
                    records: vec![held(start..bytes.len(), 1, 1)],
                    damage: vec![damage(0..start, None, Problem::Header)],
                    torn: None,
                })
            );
        }
    }

    /// Verifies: SEC-TM-024
    #[test]
    fn keeps_only_the_events_of_the_stream_numbered_in_order() {
        let pieces = [
            october(),
            // Not a number and an event.
            frame(b"junk"),
            // An event of another stream.
            record(1, &play(1, BOB, 1_000)),
            record(5, &play(5, ALICE, 1_000)),
            // The same number again, and an earlier one.
            record(5, &play(6, ALICE, 1_000)),
            record(4, &play(4, ALICE, 1_000)),
            record(6, &play(7, ALICE, 1_000)),
            // A second header is not an event either.
            october(),
        ];
        let mut ends = Vec::new();
        let mut bytes = Vec::new();
        for piece in &pieces {
            bytes.extend(piece);
            ends.push(bytes.len());
        }
        assert_eq!(
            scanned(&bytes, None),
            Ok(Scan {
                records: vec![held(ends[2]..ends[3], 5, 5), held(ends[5]..ends[6], 6, 7)],
                damage: vec![
                    damage(ends[0]..ends[1], None, Problem::NotAnEvent),
                    damage(ends[1]..ends[2], None, Problem::Foreign),
                    damage(ends[3]..ends[4], Some(5), Problem::OutOfOrder),
                    damage(ends[4]..ends[5], Some(5), Problem::OutOfOrder),
                    damage(ends[6]..ends[7], Some(6), Problem::NotAnEvent),
                ],
                torn: None,
            })
        );
        // The numbers of an earlier segment count too.
        assert_eq!(
            scanned(&bytes, Some(5)),
            Ok(Scan {
                records: vec![held(ends[5]..ends[6], 6, 7)],
                damage: vec![
                    damage(ends[0]..ends[1], Some(5), Problem::NotAnEvent),
                    damage(ends[1]..ends[2], Some(5), Problem::Foreign),
                    damage(ends[2]..ends[3], Some(5), Problem::OutOfOrder),
                    damage(ends[3]..ends[4], Some(5), Problem::OutOfOrder),
                    damage(ends[4]..ends[5], Some(5), Problem::OutOfOrder),
                    damage(ends[6]..ends[7], Some(6), Problem::NotAnEvent),
                ],
                torn: None,
            })
        );
    }

    #[test]
    fn a_record_of_a_newer_format_stops_the_scan() {
        let newer = frame_version(2, b"a record this version cannot read");
        let start = october().len();
        let bytes = [october(), newer.clone()].concat();
        assert_eq!(
            scanned(&bytes, None),
            Err(LogError::NewerRecord {
                stream: ALICE,
                month: of(2026, 10),
                offset: start,
                version: 2,
            })
        );
        assert_eq!(
            scanned(&newer, None),
            Err(LogError::NewerRecord {
                stream: ALICE,
                month: of(2026, 10),
                offset: 0,
                version: 2,
            })
        );
    }

    #[test]
    fn a_scan_out_of_steps_reports_the_rest_as_unreadable() {
        let first = record(1, &play(1, ALICE, 1_000));
        let start = october().len();
        let bytes = [october(), first].concat();
        // One step for the header's offset and one for each octet under its
        // checksum, the version and the 19 octets of its payload: 21.
        let mut enough_for_the_header = Budget::for_input(0, 0, 21);
        assert_eq!(
            scan(
                ALICE,
                of(2026, 10),
                &bytes,
                None,
                &mut enough_for_the_header
            ),
            Ok(Scan {
                records: vec![],
                damage: vec![damage(start..bytes.len(), None, Problem::Unreadable)],
                torn: None,
            })
        );
        let mut none = Budget::for_input(0, 0, 0);
        assert_eq!(
            scan(ALICE, of(2026, 10), &bytes, None, &mut none),
            Ok(Scan {
                records: vec![],
                damage: vec![damage(0..bytes.len(), None, Problem::Unreadable)],
                torn: None,
            })
        );
        // The documented budget covers a whole segment eight times over.
        assert_eq!(STEPS_PER_OCTET, 8);
        assert_eq!(
            budget(&bytes).remaining(),
            8 * u64::try_from(bytes.len()).expect("a short segment")
        );
    }
}
