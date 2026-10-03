//! Erasure selectors and their match against an event (ADR 3, section 8).
//!
//! There is no removal event: erasing history removes it. A selector names
//! what an erasure removes without holding any of it, only IDs and clock
//! values. The writer (WP-068) appends each selector to the erasure ledger,
//! drops every event in the stream that a selector covers, and checks every
//! later append against the ledger, so an erased event never comes back,
//! whatever order events arrive in.
//!
//! [`Selector::covers`] is the match on one event's envelope. One rule
//! needs the order of appends, which an event does not carry, so it stays
//! with the writer: a range or up-to selector covers an imported listen
//! only when the listen's batch record was appended before the selector.

use super::event::{DeviceId, Event, EventId, Stream};
use super::hlc::Hlc;
use super::merge::EventSet;

/// What one erasure removes from one stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selector {
    /// The stream it removes from.
    pub stream: Stream,
    /// What it removes there.
    pub scope: Scope,
}

/// What a selector removes from its stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// One history event.
    Event(EventId),
    /// The history whose clock is from `from` to `to`, both included.
    Range {
        /// The earliest clock removed.
        from: Hlc,
        /// The latest clock removed.
        to: Hlc,
    },
    /// The history whose clock is at or before this one.
    UpTo(Hlc),
    /// Everything one import batch brought in: every event whose device is
    /// the batch.
    Batch(DeviceId),
    /// The whole stream, after an account's deletion grace period.
    Stream,
}

impl Selector {
    /// Whether this selector removes `event`.
    ///
    /// The event, range and up-to scopes remove history only (plays,
    /// skips and positions); loves, settings and documents leave with the
    /// whole stream.
    #[must_use]
    pub fn covers(&self, event: &Event) -> bool {
        let history = event.body.is_history();
        event.stream == self.stream
            && match self.scope {
                Scope::Event(id) => history && event.id == id,
                Scope::Range { from, to } => history && (from..=to).contains(&event.clock),
                Scope::UpTo(to) => history && event.clock <= to,
                Scope::Batch(batch) => event.device == batch,
                Scope::Stream => true,
            }
    }
}

/// Removes from `events` every event `selector` covers.
pub fn erase(events: &mut EventSet, selector: &Selector) {
    events.retain(|event| !selector.covers(event));
}

#[cfg(test)]
mod tests {
    use super::super::event::{Body, ContentId, ItemRef, Place, Play, Position, ProfileId, Skip};
    use super::super::strategies;
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;

    const ALICE: Stream = Stream::Profile(ProfileId::new([1; 16]));
    const BOB: Stream = Stream::Profile(ProfileId::new([2; 16]));
    const PHONE: DeviceId = DeviceId::new([5; 16]);
    const IMPORT: DeviceId = DeviceId::new([6; 16]);
    const SONG: ContentId = ContentId::new([7; 32]);

    fn event(id: u8, clock: Hlc, device: DeviceId, stream: Stream, body: Body) -> Event {
        Event {
            id: EventId::new([id; 16]),
            clock,
            device,
            stream,
            body,
        }
    }

    fn play(id: u8, wall_ms: u64) -> Event {
        let body = Body::Play(Play {
            item: SONG,
            position_ms: 1_000,
            completed: true,
        });
        event(id, Hlc::new(wall_ms, 0), PHONE, ALICE, body)
    }

    fn skip(id: u8, wall_ms: u64) -> Event {
        let body = Body::Skip(Skip {
            item: SONG,
            position_ms: 1_000,
        });
        event(id, Hlc::new(wall_ms, 0), PHONE, ALICE, body)
    }

    fn position(id: u8, wall_ms: u64) -> Event {
        let body = Body::Position(Position {
            item: SONG,
            place: Place::Time { offset_ms: 1_000 },
        });
        event(id, Hlc::new(wall_ms, 0), PHONE, ALICE, body)
    }

    fn love(id: u8, wall_ms: u64) -> Event {
        let body = Body::Love(ItemRef::Content(SONG));
        event(id, Hlc::new(wall_ms, 0), PHONE, ALICE, body)
    }

    fn alice(scope: Scope) -> Selector {
        Selector {
            stream: ALICE,
            scope,
        }
    }

    #[test]
    fn an_event_selector_covers_that_history_event_only() {
        let selector = alice(Scope::Event(EventId::new([1; 16])));
        assert!(selector.covers(&play(1, 10)));
        assert!(selector.covers(&skip(1, 10)));
        assert!(selector.covers(&position(1, 10)));
        assert!(!selector.covers(&play(2, 10)));
        assert!(!selector.covers(&love(1, 10)));
    }

    #[test]
    fn a_range_covers_history_with_a_clock_inside_it() {
        let selector = alice(Scope::Range {
            from: Hlc::new(10, 1),
            to: Hlc::new(20, 0),
        });
        let covered: Vec<bool> = [
            play(1, 10),
            skip(2, 20),
            play(3, 15),
            play(4, 21),
            love(5, 15),
            position(6, 15),
        ]
        .iter()
        .map(|event| selector.covers(event))
        .collect();
        assert_eq!(covered, [false, true, true, false, false, true]);
        let mut first = play(6, 10);
        first.clock = Hlc::new(10, 1);
        assert!(selector.covers(&first));
    }

    #[test]
    fn up_to_covers_history_at_or_before_its_clock() {
        let selector = alice(Scope::UpTo(Hlc::new(20, 0)));
        let covered: Vec<bool> = [
            play(1, 0),
            skip(2, 20),
            play(3, 21),
            love(4, 5),
            position(5, 20),
            position(6, 21),
        ]
        .iter()
        .map(|event| selector.covers(event))
        .collect();
        assert_eq!(covered, [true, true, false, false, true, false]);
    }

    #[test]
    fn a_batch_covers_everything_the_import_brought_in() {
        let selector = alice(Scope::Batch(IMPORT));
        let mut imported = play(1, 10);
        imported.device = IMPORT;
        let mut record = love(2, 10);
        record.device = IMPORT;
        assert!(selector.covers(&imported));
        assert!(selector.covers(&record));
        assert!(!selector.covers(&play(3, 10)));
    }

    #[test]
    fn a_stream_selector_covers_everything_in_the_stream() {
        let selector = alice(Scope::Stream);
        assert!(selector.covers(&play(1, 10)));
        assert!(selector.covers(&love(2, 10)));
    }

    #[test]
    fn a_selector_never_covers_another_stream() {
        let mut bobs = play(1, 10);
        bobs.stream = BOB;
        let scopes = [
            Scope::Event(EventId::new([1; 16])),
            Scope::Range {
                from: Hlc::ZERO,
                to: Hlc::new(u64::MAX, u32::MAX),
            },
            Scope::UpTo(Hlc::new(u64::MAX, u32::MAX)),
            Scope::Batch(PHONE),
            Scope::Stream,
        ];
        for scope in scopes {
            assert!(!alice(scope).covers(&bobs));
            assert!(alice(scope).covers(&play(1, 10)));
        }
    }

    /// The events a writer holds after `steps`, applying each as it comes:
    /// an append is dropped when a selector already in the ledger covers
    /// it, and an erasure removes what it covers and joins the ledger.
    fn replay(steps: &[Step]) -> EventSet {
        let mut held = EventSet::new();
        let mut ledger: Vec<Selector> = Vec::new();
        for step in steps {
            match step {
                Step::Append(event) => {
                    if !ledger.iter().any(|selector| selector.covers(event)) {
                        held.insert(event.clone());
                    }
                }
                Step::Erase(selector) => {
                    erase(&mut held, selector);
                    ledger.push(*selector);
                }
            }
        }
        held
    }

    /// One step of a writer's work.
    #[derive(Debug, Clone)]
    enum Step {
        Append(Event),
        Erase(Selector),
    }

    #[test]
    fn an_erased_play_stays_erased_whatever_the_arrival_order() {
        let selector = alice(Scope::Event(EventId::new([1; 16])));
        let kept = play(2, 20);
        let before = [
            Step::Append(play(1, 10)),
            Step::Append(kept.clone()),
            Step::Erase(selector),
        ];
        let after = [
            Step::Erase(selector),
            Step::Append(kept.clone()),
            Step::Append(play(1, 10)),
        ];
        let expected: EventSet = [kept].into_iter().collect();
        assert_eq!(replay(&before), expected);
        assert_eq!(replay(&after), expected);
    }

    fn scope() -> impl Strategy<Value = Scope> {
        prop_oneof![
            strategies::event_id().prop_map(Scope::Event),
            (strategies::clock(), strategies::clock())
                .prop_map(|(from, to)| Scope::Range { from, to }),
            strategies::clock().prop_map(Scope::UpTo),
            strategies::device().prop_map(Scope::Batch),
            Just(Scope::Stream),
        ]
    }

    fn step() -> impl Strategy<Value = Step> {
        prop_oneof![
            4 => strategies::event().prop_map(Step::Append),
            1 => (strategies::stream(), scope())
                .prop_map(|(stream, scope)| Step::Erase(Selector { stream, scope })),
        ]
    }

    /// `steps` with every append of an event ID after its first replaced
    /// by a retry of the first: the writer refuses a second body under one
    /// ID (ADR 3, section 6), so a log never holds one.
    fn one_body_per_id(steps: Vec<Step>) -> Vec<Step> {
        let mut first: Vec<Event> = Vec::new();
        steps
            .into_iter()
            .map(|step| match step {
                Step::Append(event) => {
                    let earlier = first.iter().find(|seen| seen.id == event.id).cloned();
                    Step::Append(earlier.unwrap_or_else(|| {
                        first.push(event.clone());
                        event
                    }))
                }
                erase @ Step::Erase(_) => erase,
            })
            .collect()
    }

    proptest! {
        /// Any interleaving of appends, retries and erasures ends in the
        /// state of the log with every erasure applied to everything
        /// appended.
        #[test]
        fn any_interleaving_ends_in_the_filtered_log(
            steps in vec(step(), 0..16).prop_map(one_body_per_id),
        ) {
            let mut appended = EventSet::new();
            let mut selectors = Vec::new();
            for step in &steps {
                match step {
                    Step::Append(event) => appended.insert(event.clone()),
                    Step::Erase(selector) => selectors.push(*selector),
                }
            }
            appended.retain(|event| !selectors.iter().any(|selector| selector.covers(event)));
            prop_assert_eq!(replay(&steps), appended);
        }
    }
}
