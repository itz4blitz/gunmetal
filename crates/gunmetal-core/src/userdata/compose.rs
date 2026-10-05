//! The event builder: how a device makes a user event (ADR 3, section 4).
//!
//! A device authors each event itself. It draws the event's ID, so a retry
//! carries the same ID and is stored once, and it stamps the event with its
//! own hybrid logical clock, so "the latest wins" means the same on every
//! device (API-LOG-01, CLI-093). [`compose`] is that step as one pure
//! function, so no client writes it a second time.
//!
//! The core reads no clock and no random source. The caller passes the wall
//! time and sixteen random octets in as plain values, and keeps the clock
//! [`compose`] hands back for the next event it authors.
//!
//! The builder adds nothing to the body it is given: the event holds the ID
//! the caller drew, the advanced clock, the device, the profile's stream
//! and that body. A play or a skip it makes therefore holds exactly the
//! fields SEC-PRV-002 allows. Whether the event is recorded at all is not
//! decided here: every event goes on to the sink ([`super::sink`]), which
//! drops listening from a private session.
//!
//! The builder puts no rule on the body either. What a device may author is
//! the server's to check when the event arrives, because a client is never
//! trusted to enforce it (boundary TB4).

use super::event::{Body, DeviceId, Event, EventId, ProfileId, Stream};
use super::hlc::{ClockError, Hlc};

/// Who authors an event: the device it is made on, and the profile signed
/// in there.
///
/// A device writes only to a profile's stream. The household stream is
/// written by the server, for an administrator and in server order (ADR 3,
/// section 3), so an author cannot name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Author {
    /// The device the event is made on.
    pub device: DeviceId,
    /// The profile whose stream the event belongs to.
    pub profile: ProfileId,
}

/// Makes the event that says `body` happened, authored by `by`.
///
/// `clock` is the last clock the device issued or received, `wall_ms` is
/// its wall time in milliseconds since the Unix epoch, UTC, and `entropy`
/// is sixteen octets from the platform's cryptographic random source, drawn
/// for this event alone.
///
/// The event's ID is `entropy` as it stands. Its clock is `clock` advanced
/// as [`Hlc::send`] advances it: to `wall_ms` when that is later, and
/// otherwise one tick on, so the event is later than every event the device
/// has made or seen even when its wall clock was set back.
///
/// Returns the event and its clock. The caller keeps that clock as the
/// device's clock, and passes it in for the next event.
///
/// The same arguments always give the same event.
///
/// # Errors
///
/// Returns [`ClockError::Exhausted`] only when `clock` is the last clock
/// there is, which no wall time reaches. Nothing is made then.
pub fn compose(
    body: Body,
    by: Author,
    clock: Hlc,
    wall_ms: u64,
    entropy: [u8; 16],
) -> Result<(Event, Hlc), ClockError> {
    // Red step: an event that holds the body and nothing else the caller
    // gave, at a clock that does not move.
    let _ = (by, wall_ms, entropy);
    let event = Event {
        id: EventId::new([0; 16]),
        clock,
        device: DeviceId::new([0; 16]),
        stream: Stream::Household,
        body,
    };
    Ok((event, clock))
}

#[cfg(test)]
mod tests {
    use super::super::codec;
    use super::super::event::{ContentId, DocumentId, ItemRef, Play, Skip};
    use super::super::strategies;
    use super::*;
    use proptest::prelude::*;

    const PHONE: DeviceId = DeviceId::new([0x22; 16]);
    const ALICE: ProfileId = ProfileId::new([0x33; 16]);
    const BY: Author = Author {
        device: PHONE,
        profile: ALICE,
    };
    const SONG: ContentId = ContentId::new([0x44; 32]);

    /// Sixteen octets a caller drew, each one different, so an ID that held
    /// them in another order would show.
    const DRAWN: [u8; 16] = [
        0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xab, 0xac, 0xad, 0xae,
        0xaf,
    ];

    /// The wall time of the examples: 2023-11-14 22:13:20.123 UTC.
    const NOW_MS: u64 = 1_700_000_000_123;

    /// A play of the song to its end, 215 seconds in.
    fn played() -> Body {
        Body::Play(Play {
            item: SONG,
            position_ms: 215_000,
            completed: true,
        })
    }

    /// The event the builder must make of [`played`] by [`BY`] with the ID
    /// [`DRAWN`], when the advanced clock is `clock`.
    fn played_at(clock: Hlc) -> Event {
        Event {
            id: EventId::new(DRAWN),
            clock,
            device: PHONE,
            stream: Stream::Profile(ALICE),
            body: played(),
        }
    }

    /// The parts of an encoding, joined.
    fn join(parts: &[&[u8]]) -> Vec<u8> {
        parts.concat()
    }

    /// What WP-034's encoding holds before the body type, for an event with
    /// the ID [`DRAWN`], a clock at [`NOW_MS`] with `counter`, the device
    /// `22…` and the stream of profile `33…`.
    fn envelope(counter: u8) -> Vec<u8> {
        join(&[
            &DRAWN,
            // 1,700,000,000,123 ms as a varint, then the counter.
            &[0xfb, 0xd0, 0x95, 0xff, 0xbc, 0x31],
            &[counter],
            &[0x22; 16],
            // A profile's stream: tag 0, then the profile ID.
            &[0x00],
            &[0x33; 16],
        ])
    }

    /// A play the builder makes, written as the payload of a log record, is
    /// the literal encoding WP-034 fixes, octet for octet: the ID the caller
    /// drew, the clock (UTC milliseconds and a counter), the device ID, the
    /// profile ID, the body type, the item's content identity, the position
    /// and the completion state. The builder adds no field, so nothing in it
    /// could carry an address, a location, a user agent, a title or a path.
    ///
    /// Verifies: SEC-PRV-002
    #[test]
    fn a_composed_play_serialises_to_the_literal_encoding_of_the_allowed_fields() {
        let written = compose(played(), BY, Hlc::new(NOW_MS, 1), NOW_MS, DRAWN)
            .map(|(event, _)| codec::encode(&event));
        let bytes = join(&[
            &envelope(2),
            // Type 1 (play), version 1, skippable.
            &[0x01, 0x01, 0x01],
            // The body's length: 32 + 3 + 1.
            &[36],
            // The item's content identity.
            &[0x44; 32],
            // Played 215,000 ms, to the end.
            &[0xd8, 0x8f, 0x0d],
            &[0x01],
        ]);
        assert_eq!(written, Ok(bytes));
    }

    /// As for a play: a skip the builder makes holds the envelope, the
    /// item's content identity and the position, and nothing else.
    ///
    /// Verifies: SEC-PRV-002
    #[test]
    fn a_composed_skip_serialises_to_the_literal_encoding_of_the_allowed_fields() {
        let skipped = Body::Skip(Skip {
            item: SONG,
            position_ms: 300,
        });
        let written =
            compose(skipped, BY, Hlc::ZERO, NOW_MS, DRAWN).map(|(event, _)| codec::encode(&event));
        let bytes = join(&[
            &envelope(0),
            // Type 2 (skip), version 1, skippable.
            &[0x02, 0x01, 0x01],
            // The body's length: 32 + 2.
            &[34],
            // The item's content identity.
            &[0x44; 32],
            // Skipped 300 ms in.
            &[0xac, 0x02],
        ]);
        assert_eq!(written, Ok(bytes));
    }

    #[test]
    fn the_event_holds_the_drawn_id_the_device_the_profiles_stream_and_the_body() {
        let loved = Body::Love(ItemRef::Document(DocumentId::new([0x66; 16])));
        assert_eq!(
            compose(loved.clone(), BY, Hlc::new(5, 7), 9, DRAWN),
            Ok((
                Event {
                    id: EventId::new(DRAWN),
                    clock: Hlc::new(9, 0),
                    device: PHONE,
                    stream: Stream::Profile(ALICE),
                    body: loved,
                },
                Hlc::new(9, 0),
            ))
        );
    }

    /// The builder reads no clock and no random source: what it makes
    /// depends on its arguments alone.
    #[test]
    fn the_same_inputs_give_the_same_event() {
        let first = compose(played(), BY, Hlc::new(NOW_MS, 1), NOW_MS, DRAWN);
        let again = compose(played(), BY, Hlc::new(NOW_MS, 1), NOW_MS, DRAWN);
        assert_eq!(
            first,
            Ok((played_at(Hlc::new(NOW_MS, 2)), Hlc::new(NOW_MS, 2)))
        );
        assert_eq!(again, first);
    }

    #[test]
    fn other_random_octets_change_only_the_id() {
        let mut expected = played_at(Hlc::new(NOW_MS, 2));
        expected.id = EventId::new([0x5c; 16]);
        assert_eq!(
            compose(played(), BY, Hlc::new(NOW_MS, 1), NOW_MS, [0x5c; 16]),
            Ok((expected, Hlc::new(NOW_MS, 2)))
        );
    }

    /// The event's clock is the device's clock advanced as `Hlc::send`
    /// advances it, case by case, and it is the clock handed back.
    #[test]
    fn the_clock_advances_as_send_does() {
        let cases = [
            // A device's first event, at the start of time: one tick on.
            (Hlc::ZERO, 0, Hlc::new(0, 1)),
            // A later wall time: that wall time, with a zero counter.
            (Hlc::new(1_000, 4), 1_001, Hlc::new(1_001, 0)),
            // The same wall time: one tick on.
            (Hlc::new(1_000, 4), 1_000, Hlc::new(1_000, 5)),
            // A wall clock that was set back: still one tick on.
            (Hlc::new(1_000, 4), 10, Hlc::new(1_000, 5)),
            // A full counter carries into the wall time.
            (Hlc::new(1_000, u32::MAX), 1_000, Hlc::new(1_001, 0)),
            // The last clock but one advances to the last.
            (
                Hlc::new(u64::MAX, u32::MAX - 1),
                u64::MAX,
                Hlc::new(u64::MAX, u32::MAX),
            ),
        ];
        for (clock, wall_ms, next) in cases {
            assert_eq!(
                compose(played(), BY, clock, wall_ms, DRAWN),
                Ok((played_at(next), next))
            );
        }
    }

    #[test]
    fn the_last_clock_makes_no_event() {
        assert_eq!(
            compose(played(), BY, Hlc::new(u64::MAX, u32::MAX), u64::MAX, DRAWN),
            Err(ClockError::Exhausted)
        );
    }

    /// A device keeps the clock the builder hands back, so each event it
    /// makes is later than the one before, even when its wall clock is set
    /// back between them. The later of two loves is then the later one made.
    #[test]
    fn a_second_event_is_later_than_the_first_when_the_wall_clock_is_set_back() {
        let (first, kept) = compose(played(), BY, Hlc::ZERO, 5_000, DRAWN).unwrap();
        let (second, _) = compose(played(), BY, kept, 4_000, [0x5c; 16]).unwrap();
        assert_eq!(
            (first.clock, kept, second.clock),
            (Hlc::new(5_000, 0), Hlc::new(5_000, 0), Hlc::new(5_000, 1))
        );
    }

    /// Clocks near the start and the end of time, with counters near empty
    /// and near full, so ticks and carries are reached; never the last
    /// clock, which makes no event and is tested above.
    fn device_clock() -> impl Strategy<Value = Hlc> {
        (
            prop_oneof![0_u64..4, u64::MAX - 3..=u64::MAX],
            prop_oneof![0_u32..3, u32::MAX - 2..=u32::MAX],
        )
            .prop_map(|(wall_ms, logical)| Hlc::new(wall_ms, logical))
            .prop_filter("not the last clock", |clock| {
                *clock != Hlc::new(u64::MAX, u32::MAX)
            })
    }

    /// Wall times before, at and after the clocks of [`device_clock`].
    fn wall_time() -> impl Strategy<Value = u64> {
        prop_oneof![0_u64..6, u64::MAX - 5..=u64::MAX]
    }

    proptest! {
        /// Whatever the body, the author, the clock, the wall time and the
        /// random octets, the builder makes the same event each time: the
        /// drawn ID, the device, the profile's stream and the body as they
        /// were given, at the clock it hands back, which is later than the
        /// device's clock and never behind its wall time.
        #[test]
        fn any_event_holds_what_the_caller_gave_at_a_later_clock(
            body in strategies::body(),
            device in any::<[u8; 16]>(),
            profile in any::<[u8; 16]>(),
            clock in device_clock(),
            wall_ms in wall_time(),
            entropy in any::<[u8; 16]>(),
        ) {
            let by = Author {
                device: DeviceId::new(device),
                profile: ProfileId::new(profile),
            };
            let composed = compose(body.clone(), by, clock, wall_ms, entropy);
            let again = compose(body.clone(), by, clock, wall_ms, entropy);
            prop_assert_eq!(&composed, &again);
            let (event, next) = composed.unwrap();
            let expected = Event {
                id: EventId::new(entropy),
                clock: next,
                device: DeviceId::new(device),
                stream: Stream::Profile(ProfileId::new(profile)),
                body,
            };
            prop_assert_eq!(event, expected);
            prop_assert!(next > clock && next.wall_ms() >= wall_ms);
        }
    }
}
