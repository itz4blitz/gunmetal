//! The event sink: what a device may record, by the mode of its session
//! (api-needs.md, "Private listening in the synced copy").
//!
//! Every event a device makes passes through [`admit`] on its way to the
//! outbound queue. In [`Mode::Normal`] the sink hands the event back for
//! the queue. In [`Mode::Private`] it hands back nothing for what the
//! person listened to, so nothing from a private session is stored for
//! later upload, even if the device goes offline and reconnects after the
//! session ends (SEC-PRV-024, API-USR-07). The rule is written here once,
//! so no client writes it again.
//!
//! What a private session drops is history: a play, whether or not it
//! reached the end (a completion is a play with its `completed` flag set),
//! a skip, and a typed position (LAT-006), which says what the person heard
//! and when as a play does (ADR 3, section 8). The first log version has no
//! event of its own for a recommendation signal: counts, last-played times
//! and what was skipped are derived from these events, so dropping the
//! events drops the signals.
//!
//! What it keeps is what the person does on purpose, which is not history:
//! a love or an unlove, a setting, and an operation on a document or a
//! snapshot of one. The queue and its position are a document, and still
//! reach the person's own devices in a private session, so handing playback
//! to another device keeps working.
//!
//! A body of a type this version does not know is dropped in a private
//! session as well. The sink cannot tell whether it is history, and a
//! device never authors a type its own core does not know, so refusing it
//! costs nothing and keeps the promise.
//!
//! The sink is the device's half of the control. The server refuses a play
//! from a private session whatever a client sends (WP-086), because a
//! client is never trusted to enforce a rule (boundary TB4).

use super::event::Event;

/// Whether the session an event comes from is recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// An ordinary session: every event is recorded.
    Normal,
    /// A private session (API-USR-07): what is listened to leaves no
    /// history.
    Private,
}

/// The event to put on the outbound queue for `event`, made in a session of
/// `mode`, or `None` when that session must not record it.
///
/// [`Mode::Normal`] hands every event back unchanged. [`Mode::Private`]
/// hands back, unchanged, a love, an unlove, a setting, a document
/// operation and a document snapshot, and drops a play, a skip, a position
/// and a body of a type this version does not know.
#[must_use]
pub fn admit(event: Event, mode: Mode) -> Option<Event> {
    // Red step: the two modes the wrong way round, and no rule for the body.
    match mode {
        Mode::Normal => None,
        Mode::Private => Some(event),
    }
}

#[cfg(test)]
mod tests {
    use super::super::event::{
        Body, BodyType, ContentId, DeviceId, DocumentId, DocumentKind, DocumentOp, DocumentRef,
        DocumentSnapshot, EventId, ItemRef, Place, Play, Position, ProfileId, Setting, SettingKey,
        SettingScope, SettingValue, Skip, Stream, UnknownBody,
    };
    use super::super::hlc::Hlc;
    use super::super::strategies;
    use super::*;
    use crate::untrusted::Untrusted;
    use proptest::prelude::*;

    const SONG: ContentId = ContentId::new([7; 32]);

    /// An event of profile `1…`'s, made on device `5…`, that says `body`
    /// happened.
    fn event(body: Body) -> Event {
        Event {
            id: EventId::new([9; 16]),
            clock: Hlc::new(1_700_000_000_123, 2),
            device: DeviceId::new([5; 16]),
            stream: Stream::Profile(ProfileId::new([1; 16])),
            body,
        }
    }

    /// A play of the song: one that stopped part-way, or one that reached
    /// the end, which is a completion.
    fn play(completed: bool) -> Body {
        Body::Play(Play {
            item: SONG,
            position_ms: 215_000,
            completed,
        })
    }

    fn skip() -> Body {
        Body::Skip(Skip {
            item: SONG,
            position_ms: 300,
        })
    }

    fn position() -> Body {
        Body::Position(Position {
            item: SONG,
            place: Place::Time { offset_ms: 90_000 },
        })
    }

    fn love() -> Body {
        Body::Love(ItemRef::Content(SONG))
    }

    fn unlove() -> Body {
        Body::Unlove(ItemRef::Document(DocumentId::new([6; 16])))
    }

    fn setting() -> Body {
        Body::Setting(Setting {
            scope: SettingScope::Person,
            key: SettingKey::new(Untrusted::new("playback.crossfade")).unwrap(),
            value: SettingValue::new(Untrusted::new(&[1, 2])).unwrap(),
        })
    }

    fn queue() -> DocumentRef {
        DocumentRef {
            kind: DocumentKind::Queue,
            id: DocumentId::new([8; 16]),
        }
    }

    fn operation() -> Body {
        Body::DocumentOp(DocumentOp {
            document: queue(),
            based_on: 3,
            payload: vec![1, 2],
        })
    }

    fn snapshot() -> Body {
        Body::DocumentSnapshot(DocumentSnapshot {
            document: queue(),
            version: 4,
            payload: vec![3],
        })
    }

    /// A body of a type a later release added.
    fn unknown() -> Body {
        let later = BodyType {
            tag: 40,
            version: 1,
            skippable: true,
        };
        Body::Unknown(UnknownBody::new(later, vec![1, 2, 3]))
    }

    #[test]
    fn normal_mode_hands_every_event_back_for_the_queue() {
        let bodies = [
            play(false),
            play(true),
            skip(),
            position(),
            love(),
            unlove(),
            setting(),
            operation(),
            snapshot(),
            unknown(),
        ];
        for body in bodies {
            assert_eq!(admit(event(body.clone()), Mode::Normal), Some(event(body)));
        }
    }

    /// A private session records nothing the person listened to: not a
    /// play, whether it stopped part-way or reached the end, not a skip and
    /// not a position. The sink hands back nothing to queue for them.
    ///
    /// Verifies: SEC-PRV-024
    #[test]
    fn private_mode_drops_every_play_completion_skip_and_position() {
        let listening = [
            ("a play that stopped part-way", play(false)),
            ("a play that reached the end", play(true)),
            ("a skip", skip()),
            ("a position", position()),
        ];
        for (what, body) in listening {
            assert_eq!(admit(event(body), Mode::Private), None, "{what}");
        }
    }

    /// api-needs.md lists plays, skips, completions and recommendation
    /// signals as dropped, not loves: a heart pressed in a private session
    /// is meant.
    #[test]
    fn private_mode_hands_back_a_love_and_an_unlove() {
        assert_eq!(admit(event(love()), Mode::Private), Some(event(love())));
        assert_eq!(admit(event(unlove()), Mode::Private), Some(event(unlove())));
    }

    /// Settings and documents are not history. The queue and its position
    /// are a document, and still reach the person's own devices.
    #[test]
    fn private_mode_hands_back_settings_and_documents() {
        for body in [setting(), operation(), snapshot()] {
            assert_eq!(admit(event(body.clone()), Mode::Private), Some(event(body)));
        }
    }

    /// The sink cannot tell whether a type it does not know is history, so
    /// a private session does not record it.
    #[test]
    fn private_mode_drops_a_body_of_a_type_this_version_does_not_know() {
        assert_eq!(admit(event(unknown()), Mode::Private), None);
    }

    /// What a private session let through of `event` that it must not:
    /// history, which is a play, whether or not it reached the end, a skip
    /// or a position (ADR 3, section 8). `None` when the sink is right.
    fn leaked(event: Event) -> Option<Event> {
        admit(event, Mode::Private).filter(|kept| kept.body.is_history())
    }

    /// The check the property below makes, on an event the sink keeps and
    /// on one it drops, so every line of it runs on every run.
    #[test]
    fn nothing_leaks_from_a_love_or_a_play() {
        assert_eq!(leaked(event(love())), None);
        assert_eq!(leaked(event(play(true))), None);
    }

    proptest! {
        /// Nothing a private session admits is a play, a skip, a completion
        /// or a position, whatever the event.
        ///
        /// Verifies: SEC-PRV-024
        #[test]
        fn nothing_admitted_in_private_mode_is_history(event in strategies::event()) {
            prop_assert_eq!(leaked(event), None);
        }

        /// A private session hands an event back as it was given or not at
        /// all; it never alters one.
        #[test]
        fn private_mode_admits_an_event_unchanged_or_not_at_all(
            event in strategies::event(),
        ) {
            let kept = admit(event.clone(), Mode::Private);
            // A dropped event stands in as itself, so one comparison holds
            // both answers and no line runs on some draws only.
            prop_assert_eq!(kept.or(Some(event.clone())), Some(event));
        }

        #[test]
        fn normal_mode_admits_any_event_unchanged(event in strategies::event()) {
            prop_assert_eq!(admit(event.clone(), Mode::Normal), Some(event));
        }
    }
}
