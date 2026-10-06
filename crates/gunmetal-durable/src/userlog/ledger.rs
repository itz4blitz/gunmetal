//! The erasure ledger's file: `durable/erasure/ledger` (ADR 3, section 8).
//!
//! The ledger holds one framed record for each erasure, appended and synced
//! before any segment is rewritten. An entry names what was erased without
//! holding any of it: a stream, and an event ID, clock values or a batch
//! ID. It also keeps the sequence number the stream had reached, so that a
//! number an erased record carried is never given out again, and so that
//! the writer knows which records were appended before the erasure.
//!
//! ```text
//! floor:  u64 LE   the stream's next sequence number when it was recorded
//! stream: 0 and the profile's 16 octets, or 1 for the household
//! scope:  0 and an event ID (16 octets)
//!         1 and two clocks, the first and the last removed
//!         2 and a clock, the last removed
//!         3 and a batch ID (16 octets)
//!         4, the whole stream, which is a profile's and never the household's
//! clock:  wall time u64 LE, logical counter u32 LE
//! ```
//!
//! The household's stream is never erased whole (ADR 3, sections 3 and 8),
//! and the data root refuses to remove its directory. A promise to do so
//! could never be kept, and the log would fail on it at every start, so the
//! ledger never holds one: [`accepts`] is asked before an entry is written
//! and when one is read.
//!
//! The ledger is read back as untrusted input (boundary TB10), and exactly
//! the octets above are an entry. A ledger that cannot be read does not
//! open: an erasure it promised could otherwise be undone.

use std::ops::Range;

use gunmetal_core::logframe::{self, Item, RECORD_VERSION};
use gunmetal_core::parse::{Budget, Cursor};
use gunmetal_core::userdata::erasure::{Scope, Selector};
use gunmetal_core::userdata::event::{DeviceId, EventId, ProfileId, Stream};
use gunmetal_core::userdata::hlc::Hlc;
use gunmetal_fs::path::{DataDir, DataPath};

use crate::userlog::error::LedgerFlaw;

/// The ledger's directory.
pub(crate) const DIR: DataPath = DataPath::constant(DataDir::Durable, "erasure");

/// The ledger.
pub(crate) const FILE: DataPath = DataPath::constant(DataDir::Durable, "erasure/ledger");

/// The octets of a record around its payload.
const FRAME: usize = 9;

/// One erasure: what it removes, and the sequence number its stream had
/// reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Entry {
    /// What the erasure removes.
    pub(crate) selector: Selector,
    /// The stream's next sequence number when the erasure was recorded.
    pub(crate) floor: u64,
}

/// What reading the ledger found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Parsed {
    /// The entries, in the order they were recorded.
    pub(crate) entries: Vec<Entry>,
    /// Octets at the end that are not a whole record: an entry a crash cut
    /// off before it was synced, so the erasure was never promised.
    pub(crate) torn: Option<Range<usize>>,
}

/// The one erasure the ledger never holds: the household's whole stream.
const HOUSEHOLD: Selector = Selector {
    stream: Stream::Household,
    scope: Scope::Stream,
};

/// Whether the ledger may hold an erasure of `selector`: every one but the
/// household's whole stream.
pub(crate) fn accepts(selector: Selector) -> bool {
    selector != HOUSEHOLD
}

/// `entry` as one framed record.
pub(crate) fn frame(entry: &Entry) -> Vec<u8> {
    let mut payload = entry.floor.to_le_bytes().to_vec();
    match entry.selector.stream {
        Stream::Profile(profile) => {
            payload.push(0);
            payload.extend(profile.bytes());
        }
        Stream::Household => payload.push(1),
    }
    match entry.selector.scope {
        Scope::Event(id) => {
            payload.push(0);
            payload.extend(id.bytes());
        }
        Scope::Range { from, to } => {
            payload.push(1);
            put_clock(&mut payload, from);
            put_clock(&mut payload, to);
        }
        Scope::UpTo(to) => {
            payload.push(2);
            put_clock(&mut payload, to);
        }
        Scope::Batch(batch) => {
            payload.push(3);
            payload.extend(batch.bytes());
        }
        Scope::Stream => payload.push(4),
    }
    // An entry is at most 50 octets, far under a record's cap.
    logframe::encode(&payload).unwrap_or_default()
}

/// Appends a clock: its wall time, then its logical counter.
fn put_clock(out: &mut Vec<u8>, clock: Hlc) {
    out.extend(clock.wall_ms().to_le_bytes());
    out.extend(clock.logical().to_le_bytes());
}

/// Reads the ledger's octets.
///
/// # Errors
///
/// Returns the [`LedgerFlaw`] that stops the log from opening: octets
/// before the end that are not a whole record, a whole record that is not
/// an entry, or a spent step budget.
pub(crate) fn parse(bytes: &[u8], budget: &mut Budget) -> Result<Parsed, LedgerFlaw> {
    let mut parsed = Parsed {
        entries: Vec::new(),
        torn: None,
    };
    let mut pos = 0_usize;
    for item in logframe::records(bytes, budget) {
        match item.map_err(LedgerFlaw::Fault)? {
            Item::Record(record) => {
                let end = pos
                    .saturating_add(FRAME)
                    .saturating_add(record.payload.len());
                let found = (record.version == RECORD_VERSION)
                    .then_some(record.payload)
                    .and_then(entry)
                    .ok_or(LedgerFlaw::NotAnEntry { range: pos..end })?;
                parsed.entries.push(found);
                pos = end;
            }
            Item::Damaged { range } if range.end == bytes.len() => parsed.torn = Some(range),
            Item::Damaged { range } => return Err(LedgerFlaw::Damaged { range }),
        }
    }
    Ok(parsed)
}

/// The entry `payload` holds, or `None` when it is not exactly one the
/// ledger may hold.
fn entry(payload: &[u8]) -> Option<Entry> {
    let mut cursor = Cursor::new(payload);
    let floor = cursor.u64_le().ok()?;
    let stream = match cursor.u8().ok()? {
        0 => Stream::Profile(ProfileId::new(cursor.array().ok()?)),
        1 => Stream::Household,
        _ => return None,
    };
    let scope = match cursor.u8().ok()? {
        0 => Scope::Event(EventId::new(cursor.array().ok()?)),
        1 => Scope::Range {
            from: clock(&mut cursor)?,
            to: clock(&mut cursor)?,
        },
        2 => Scope::UpTo(clock(&mut cursor)?),
        3 => Scope::Batch(DeviceId::new(cursor.array().ok()?)),
        4 => Scope::Stream,
        _ => return None,
    };
    let selector = Selector { stream, scope };
    (cursor.is_empty() && accepts(selector)).then_some(Entry { selector, floor })
}

/// Reads a clock.
fn clock(cursor: &mut Cursor<'_>) -> Option<Hlc> {
    let wall_ms = cursor.u64_le().ok()?;
    cursor
        .u32_le()
        .ok()
        .map(|logical| Hlc::new(wall_ms, logical))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::userlog::testing::{ALICE, frame as framed, frame_version};
    use gunmetal_core::parse::ParseFault;

    const ID: [u8; 16] = [0x1D; 16];

    fn alice(scope: Scope, floor: u64) -> Entry {
        Entry {
            selector: Selector {
                stream: ALICE,
                scope,
            },
            floor,
        }
    }

    /// Every kind of entry with its payload, written out octet by octet.
    fn samples() -> Vec<(Entry, Vec<u8>)> {
        // Floor 258, then Alice's stream.
        let mut head = vec![0x02, 0x01, 0, 0, 0, 0, 0, 0, 0];
        head.extend([0xA1; 16]);
        // Wall time 1,000 with counter 7, and wall time 2,000 with counter 9.
        let early = [0xE8, 0x03, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0];
        let late = [0xD0, 0x07, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0];
        let with = |tail: &[&[u8]]| [head.as_slice(), tail.concat().as_slice()].concat();
        vec![
            (
                alice(Scope::Event(EventId::new(ID)), 258),
                with(&[&[0], &ID]),
            ),
            (
                alice(
                    Scope::Range {
                        from: Hlc::new(1_000, 7),
                        to: Hlc::new(2_000, 9),
                    },
                    258,
                ),
                with(&[&[1], &early, &late]),
            ),
            (
                alice(Scope::UpTo(Hlc::new(2_000, 9)), 258),
                with(&[&[2], &late]),
            ),
            (
                alice(Scope::Batch(DeviceId::new(ID)), 258),
                with(&[&[3], &ID]),
            ),
            (alice(Scope::Stream, 258), with(&[&[4]])),
            // Floor 0, the household's stream, and an import batch.
            (
                Entry {
                    selector: Selector {
                        stream: Stream::Household,
                        scope: Scope::Batch(DeviceId::new(ID)),
                    },
                    floor: 0,
                },
                [&[0, 0, 0, 0, 0, 0, 0, 0, 1, 3][..], &ID].concat(),
            ),
        ]
    }

    /// Reads `bytes` with the steps the log gives a ledger of that length.
    fn parsed(bytes: &[u8]) -> Result<Parsed, LedgerFlaw> {
        let len = u64::try_from(bytes.len()).expect("a short ledger");
        parse(bytes, &mut Budget::for_input(len, 8, 0))
    }

    #[test]
    fn writes_each_kind_of_entry_as_its_octets() {
        assert_eq!(FILE.rel(), "erasure/ledger");
        assert_eq!(DIR.rel(), "erasure");
        assert_eq!(framed(b"").len(), FRAME);
        for (entry, payload) in samples() {
            assert_eq!(frame(&entry), framed(&payload), "{entry:?}");
        }
    }

    #[test]
    fn reads_the_entries_back_in_order() {
        let bytes: Vec<u8> = samples()
            .iter()
            .flat_map(|(_, payload)| framed(payload))
            .collect();
        let entries: Vec<Entry> = samples().into_iter().map(|(entry, _)| entry).collect();
        assert_eq!(
            parsed(&bytes),
            Ok(Parsed {
                entries,
                torn: None
            })
        );
        assert_eq!(
            parsed(&[]),
            Ok(Parsed {
                entries: vec![],
                torn: None
            })
        );
    }

    #[test]
    fn an_entry_is_exactly_its_octets() {
        for (sample, payload) in samples() {
            assert_eq!(entry(&payload), Some(sample));
            for end in 0..payload.len() {
                assert_eq!(entry(&payload[..end]), None, "{sample:?} cut at {end}");
            }
            let mut longer = payload.clone();
            longer.push(0);
            assert_eq!(entry(&longer), None, "{sample:?} with an octet after it");
        }
        // A stream and a scope that have no tag.
        assert_eq!(entry(&[0, 0, 0, 0, 0, 0, 0, 0, 2, 4]), None);
        assert_eq!(entry(&[0, 0, 0, 0, 0, 0, 0, 0, 1, 5]), None);
    }

    #[test]
    fn never_holds_a_promise_to_erase_the_household_s_whole_stream() {
        let household = Selector {
            stream: Stream::Household,
            scope: Scope::Stream,
        };
        assert!(!accepts(household));
        // Every other erasure is held: a profile's whole stream, and
        // anything less of the household's.
        let held: Vec<Selector> = samples()
            .into_iter()
            .map(|(entry, _)| entry.selector)
            .filter(|selector| accepts(*selector))
            .collect();
        let all: Vec<Selector> = samples()
            .into_iter()
            .map(|(entry, _)| entry.selector)
            .collect();
        assert_eq!(held, all);
        // Its octets are not an entry: floor 258, tag 1 for the household,
        // scope 4. With them in it the ledger does not open.
        let octets = [0x02, 0x01, 0, 0, 0, 0, 0, 0, 1, 4];
        assert_eq!(entry(&octets), None);
        let record = framed(&octets);
        assert_eq!(
            parsed(&record),
            Err(LedgerFlaw::NotAnEntry {
                range: 0..record.len()
            })
        );
    }

    #[test]
    fn an_entry_cut_off_at_the_end_was_never_promised() {
        let (first, payload) = samples().remove(0);
        let whole = framed(&payload);
        let bytes = [whole.clone(), whole.clone()].concat();
        for end in whole.len() + 1..bytes.len() {
            assert_eq!(
                parsed(&bytes[..end]),
                Ok(Parsed {
                    entries: vec![first],
                    torn: Some(whole.len()..end),
                }),
                "cut at {end}"
            );
        }
    }

    #[test]
    fn a_ledger_that_cannot_be_read_is_refused() {
        let (_, payload) = samples().remove(0);
        let whole = framed(&payload);
        let len = whole.len();
        // Damage before the end.
        let mut damaged = [whole.clone(), whole.clone()].concat();
        damaged[12] ^= 0x40;
        assert_eq!(parsed(&damaged), Err(LedgerFlaw::Damaged { range: 0..len }));
        // A whole record that is not an entry, and one in a newer format.
        let junk = framed(b"junk");
        let after = [whole.clone(), junk.clone()].concat();
        assert_eq!(
            parsed(&after),
            Err(LedgerFlaw::NotAnEntry {
                range: len..len + junk.len()
            })
        );
        assert_eq!(
            parsed(&frame_version(2, &payload)),
            Err(LedgerFlaw::NotAnEntry { range: 0..len })
        );
        // No steps to read it with.
        assert_eq!(
            parse(&whole, &mut Budget::for_input(0, 0, 0)),
            Err(LedgerFlaw::Fault(ParseFault::BudgetExceeded { offset: 0 }))
        );
    }
}
