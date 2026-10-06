//! One event as a segment holds it (ADR 3, section 4).
//!
//! The writer stamps every event with its stream's next sequence number.
//! The number is not part of the event, which a device authors without one,
//! so it goes in front of the event in the record's payload:
//!
//! ```text
//! sequence: u64 LE   strictly increasing in a stream, never reused
//! event:    the octets `gunmetal_core::userdata::codec` writes
//! ```

use gunmetal_core::parse::Budget;
use gunmetal_core::untrusted::Untrusted;
use gunmetal_core::userdata::codec;
use gunmetal_core::userdata::event::Event;

/// One event of a stream, with the sequence number the writer gave it.
///
/// Projections and damage reports refer to a record by its number. Erasure
/// leaves gaps, and no number is used twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamped {
    /// The record's sequence number in its stream, from 1.
    pub seq: u64,
    /// The event.
    pub event: Event,
}

/// How many octets the sequence number takes.
const SEQ: usize = 8;

/// The payload of the record that holds `octets`, an encoded event, as
/// number `seq`.
pub(crate) fn payload(seq: u64, octets: &[u8]) -> Vec<u8> {
    let mut out = seq.to_le_bytes().to_vec();
    out.extend_from_slice(octets);
    out
}

/// The numbered event `payload` holds, or `None` when it is not exactly a
/// sequence number and one event. What comes back from storage is untrusted
/// (boundary TB10), so the event is read under the codec's step budget.
pub(crate) fn read(payload: &[u8]) -> Option<Stamped> {
    let (seq, octets) = payload.split_first_chunk::<SEQ>()?;
    let len = u64::try_from(octets.len()).unwrap_or(u64::MAX);
    let mut budget = Budget::for_input(len, codec::STEPS_PER_BYTE, codec::FIXED_STEPS);
    codec::decode(Untrusted::new(octets), &mut budget)
        .ok()
        .map(|event| Stamped {
            seq: u64::from_le_bytes(*seq),
            event,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::userlog::testing::{ALICE, play};
    use proptest::prelude::*;

    /// A play by Alice as the codec writes it, octet by octet.
    fn play_octets() -> Vec<u8> {
        let mut octets = Vec::new();
        // Event ID.
        octets.extend([7; 16]);
        // Clock: wall time 1,000 as a varint, then the counter 0.
        octets.extend([0xE8, 0x07, 0x00]);
        // Device ID.
        octets.extend([0xD0; 16]);
        // Stream: tag 0 and Alice's profile ID.
        octets.push(0);
        octets.extend([0xA1; 16]);
        // Body type: tag 1, version 1, skippable.
        octets.extend([1, 1, 1]);
        // Body: 34 octets, the content ID, a position of 30 ms, completed.
        octets.push(34);
        octets.extend([0xC0; 32]);
        octets.extend([30, 1]);
        octets
    }

    #[test]
    fn a_payload_is_the_sequence_number_then_the_event() {
        let mut expected = vec![0x2A, 0, 0, 0, 0, 0, 0, 0];
        expected.extend(play_octets());
        assert_eq!(payload(42, &play_octets()), expected);
        assert_eq!(
            payload(0x0102_0304_0506_0708, b"x"),
            [8, 7, 6, 5, 4, 3, 2, 1, b'x']
        );
    }

    #[test]
    fn reads_the_number_and_the_event_back() {
        let mut stored = vec![0x2A, 0, 0, 0, 0, 0, 0, 0];
        stored.extend(play_octets());
        assert_eq!(
            read(&stored),
            Some(Stamped {
                seq: 42,
                event: play(7, ALICE, 1_000),
            })
        );
        let mut numbered = vec![8, 7, 6, 5, 4, 3, 2, 1];
        numbered.extend(play_octets());
        assert_eq!(
            read(&numbered).map(|stamped| stamped.seq),
            Some(0x0102_0304_0506_0708)
        );
    }

    #[test]
    fn refuses_a_payload_that_is_not_a_number_and_one_event() {
        let mut stored = vec![1, 0, 0, 0, 0, 0, 0, 0];
        stored.extend(play_octets());
        // Too short for a number, a number alone, an event cut short, and an
        // event with an octet after it.
        assert_eq!(read(&[]), None);
        assert_eq!(read(&stored[..7]), None);
        assert_eq!(read(&stored[..8]), None);
        assert_eq!(read(&stored[..stored.len() - 1]), None);
        stored.push(0);
        assert_eq!(read(&stored), None);
    }

    proptest! {
        #[test]
        fn reads_exactly_what_was_written_and_nothing_cut_short(
            seq in any::<u64>(),
            id in any::<u8>(),
            wall_ms in any::<u64>(),
            cut in any::<proptest::sample::Index>(),
            short in proptest::collection::vec(any::<u8>(), 0..SEQ),
            noise in proptest::collection::vec(any::<u8>(), 0..160),
        ) {
            let event = play(id, ALICE, wall_ms);
            let stored = payload(seq, &codec::encode(&event));
            prop_assert_eq!(read(&stored), Some(Stamped { seq, event }));
            prop_assert_eq!(read(&stored[..cut.index(stored.len())]), None);
            // Nothing shorter than a number is read, and any octets at all
            // are read or refused, never a panic.
            prop_assert_eq!(read(&short), None);
            drop(read(&noise));
        }
    }
}
