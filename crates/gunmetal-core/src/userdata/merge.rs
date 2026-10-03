//! The merge rules: what a set of events means, whatever order the events
//! arrived in ([api-needs.md, "How conflicts resolve"](../../../../docs/plan/api-needs.md);
//! ADR 3, section 7).
//!
//! - **Plays and skips** are a union of every event, one per event ID.
//! - **Loves**, **settings** and **positions** take the latest clock, per
//!   item, per key and scope, or per item; a tie goes to the larger device ID, then the larger event
//!   ID, so every replica picks the same event.
//! - **Document operations and snapshots** are kept, one per event ID; the
//!   server orders them, and the documents' own modules replay them.
//! - **Bodies of unknown types** are kept and passed through unchanged.
//!
//! An [`EventSet`] holds one event per ID, so [`EventSet::merge`] is a set
//! union, and therefore commutative, associative and idempotent: two
//! replicas that hold the same events agree on every value derived from
//! them, which is what makes merging offline work safe. Two different
//! events with one ID never come from an honest device, and the writer
//! refuses the second (ADR 3, section 6); should a set meet both, it keeps
//! the larger by the order of [`Event`], so the union still does not depend
//! on order.
//!
//! The derived values read one stream's events, so a profile's counts and
//! loves never include another profile's.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use super::event::{
    Body, ContentId, Event, EventId, ItemRef, Place, SettingKey, SettingScope, SettingValue, Stream,
};
use super::hlc::Hlc;

/// Events, at most one per event ID.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventSet {
    /// The events, by ID.
    events: BTreeMap<EventId, Event>,
}

impl EventSet {
    /// An empty set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `event`. When the set already holds an event with its ID, it
    /// keeps the larger of the two by the order of [`Event`].
    pub fn insert(&mut self, event: Event) {
        match self.events.entry(event.id) {
            Entry::Vacant(slot) => {
                slot.insert(event);
            }
            Entry::Occupied(mut slot) => {
                if event.cmp(slot.get()) == Ordering::Greater {
                    slot.insert(event);
                }
            }
        }
    }

    /// The union of `self` and `other`.
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        let mut merged = self.clone();
        merged.extend(other.iter().cloned());
        merged
    }

    /// Keeps only the events for which `keep` is true.
    pub fn retain(&mut self, mut keep: impl FnMut(&Event) -> bool) {
        self.events.retain(|_, event| keep(event));
    }

    /// The events, in order of their IDs.
    pub fn iter(&self) -> impl Iterator<Item = &Event> {
        self.events.values()
    }

    /// How many events the set holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether the set holds no event.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Extend<Event> for EventSet {
    fn extend<I: IntoIterator<Item = Event>>(&mut self, events: I) {
        for event in events {
            self.insert(event);
        }
    }
}

impl FromIterator<Event> for EventSet {
    fn from_iter<I: IntoIterator<Item = Event>>(events: I) -> Self {
        let mut set = Self::new();
        set.extend(events);
        set
    }
}

/// One item's numbers (MUS-182, API-LOG-04).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemCounts {
    /// How many times it was played.
    pub plays: u64,
    /// How many times it was skipped.
    pub skips: u64,
    /// The clock of its latest play, if it was played.
    pub last_played: Option<Hlc>,
}

/// Play counts, skips and last-played times, per item.
pub type Counts = BTreeMap<ContentId, ItemCounts>;

/// The numbers of every item `stream` played or skipped in `events`.
#[must_use]
pub fn derive_counts(events: &EventSet, stream: Stream) -> Counts {
    let mut counts = Counts::new();
    for event in of_stream(events, stream) {
        let (item, played) = match &event.body {
            Body::Play(play) => (play.item, true),
            Body::Skip(skip) => (skip.item, false),
            _ => continue,
        };
        let entry = counts.entry(item).or_insert(ItemCounts {
            plays: 0,
            skips: 0,
            last_played: None,
        });
        if played {
            entry.plays = entry.plays.saturating_add(1);
            entry.last_played = entry.last_played.max(Some(event.clock));
        } else {
            entry.skips = entry.skips.saturating_add(1);
        }
    }
    counts
}

/// The value of the event that wins a "latest wins" rule among `values`:
/// the latest clock, then the larger device ID, then the larger event ID.
fn latest<'a, T>(values: impl Iterator<Item = (&'a Event, T)>) -> Option<T> {
    values
        .max_by_key(|(event, _)| (event.clock, event.device, event.id))
        .map(|(_, value)| value)
}

/// The events of `stream` in `events`.
fn of_stream(events: &EventSet, stream: Stream) -> impl Iterator<Item = &Event> {
    events.iter().filter(move |event| event.stream == stream)
}

/// Whether `stream` currently loves `item`: its latest love or unlove of
/// the item is a love. With neither, it does not.
#[must_use]
pub fn current_love(events: &EventSet, stream: Stream, item: ItemRef) -> bool {
    let toggles = of_stream(events, stream).filter_map(|event| match event.body {
        Body::Love(loved) if loved == item => Some((event, true)),
        Body::Unlove(unloved) if unloved == item => Some((event, false)),
        _ => None,
    });
    latest(toggles) == Some(true)
}

/// The place `stream` last reached in `item`: the place of its latest
/// position there, if it has one (LAT-006). As for resume points in the
/// conflict table, the latest clock wins per item; each version of an
/// item has a content identity of its own.
#[must_use]
pub fn current_position(events: &EventSet, stream: Stream, item: ContentId) -> Option<Place> {
    let positions = of_stream(events, stream).filter_map(|event| match event.body {
        Body::Position(position) if position.item == item => Some((event, position.place)),
        _ => None,
    });
    latest(positions)
}

/// The current value of the setting `key` with `scope` in `stream`: the
/// value of its latest setting, if it was ever set.
#[must_use]
pub fn current_setting<'a>(
    events: &'a EventSet,
    stream: Stream,
    scope: SettingScope,
    key: &SettingKey,
) -> Option<&'a SettingValue> {
    let settings = of_stream(events, stream).filter_map(|event| match &event.body {
        Body::Setting(setting) if setting.scope == scope && setting.key == *key => {
            Some((event, &setting.value))
        }
        _ => None,
    });
    latest(settings)
}

#[cfg(test)]
mod tests {
    use super::super::event::{
        BodyType, DeviceId, DocumentId, Play, Position, ProfileId, Setting, Skip, UnknownBody,
    };
    use super::super::strategies;
    use super::*;
    use crate::untrusted::Untrusted;
    use proptest::collection::vec;
    use proptest::prelude::*;

    const ALICE: Stream = Stream::Profile(ProfileId::new([1; 16]));
    const BOB: Stream = Stream::Profile(ProfileId::new([2; 16]));
    const SONG: ContentId = ContentId::new([7; 32]);
    const OTHER_SONG: ContentId = ContentId::new([8; 32]);

    fn at(id: u8, wall_ms: u64, device: u8, stream: Stream, body: Body) -> Event {
        Event {
            id: EventId::new([id; 16]),
            clock: Hlc::new(wall_ms, 0),
            device: DeviceId::new([device; 16]),
            stream,
            body,
        }
    }

    fn play(id: u8, wall_ms: u64, item: ContentId) -> Event {
        let body = Body::Play(Play {
            item,
            position_ms: 1_000,
            completed: true,
        });
        at(id, wall_ms, 1, ALICE, body)
    }

    fn skip(id: u8, wall_ms: u64, item: ContentId) -> Event {
        let body = Body::Skip(Skip {
            item,
            position_ms: 2_000,
        });
        at(id, wall_ms, 1, ALICE, body)
    }

    #[test]
    fn a_new_set_is_empty() {
        let set = EventSet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
        assert_eq!(set, EventSet::default());
    }

    #[test]
    fn plays_are_de_duplicated_by_event_id() {
        let first = play(1, 10, SONG);
        let second = play(2, 20, SONG);
        let set: EventSet = [first.clone(), second.clone(), first.clone()]
            .into_iter()
            .collect();
        assert_eq!(set.len(), 2);
        assert!(!set.is_empty());
        assert_eq!(set.iter().cloned().collect::<Vec<_>>(), [first, second]);
    }

    #[test]
    fn two_events_with_one_id_keep_the_larger_whatever_the_order() {
        let small = play(1, 10, SONG);
        let large = play(1, 11, SONG);
        let forwards: EventSet = [small.clone(), large.clone()].into_iter().collect();
        let backwards: EventSet = [large.clone(), small].into_iter().collect();
        assert_eq!(
            forwards.iter().cloned().collect::<Vec<_>>(),
            std::slice::from_ref(&large)
        );
        assert_eq!(backwards.iter().cloned().collect::<Vec<_>>(), [large]);
    }

    #[test]
    fn merging_takes_the_union() {
        let a: EventSet = [play(1, 10, SONG), play(2, 20, SONG)].into_iter().collect();
        let b: EventSet = [play(2, 20, SONG), play(3, 30, SONG)].into_iter().collect();
        let expected = vec![play(1, 10, SONG), play(2, 20, SONG), play(3, 30, SONG)];
        assert_eq!(a.merge(&b).iter().cloned().collect::<Vec<_>>(), expected);
    }

    #[test]
    fn retain_keeps_only_what_it_is_asked_to() {
        let mut set: EventSet = [play(1, 10, SONG), play(2, 20, SONG)].into_iter().collect();
        set.retain(|event| event.clock.wall_ms() > 10);
        assert_eq!(set.iter().cloned().collect::<Vec<_>>(), [play(2, 20, SONG)]);
    }

    #[test]
    fn counts_plays_skips_and_the_latest_play_per_item() {
        let set: EventSet = [
            play(1, 30, SONG),
            play(2, 10, SONG),
            skip(3, 50, SONG),
            skip(4, 40, OTHER_SONG),
            at(5, 60, 1, ALICE, Body::Love(ItemRef::Content(SONG))),
            at(
                6,
                70,
                1,
                BOB,
                Body::Play(Play {
                    item: SONG,
                    position_ms: 0,
                    completed: false,
                }),
            ),
        ]
        .into_iter()
        .collect();
        let expected = Counts::from([
            (
                SONG,
                ItemCounts {
                    plays: 2,
                    skips: 1,
                    last_played: Some(Hlc::new(30, 0)),
                },
            ),
            (
                OTHER_SONG,
                ItemCounts {
                    plays: 0,
                    skips: 1,
                    last_played: None,
                },
            ),
        ]);
        assert_eq!(derive_counts(&set, ALICE), expected);
        let bob = Counts::from([(
            SONG,
            ItemCounts {
                plays: 1,
                skips: 0,
                last_played: Some(Hlc::new(70, 0)),
            },
        )]);
        assert_eq!(derive_counts(&set, BOB), bob);
        assert_eq!(derive_counts(&set, Stream::Household), Counts::new());
    }

    fn love(id: u8, wall_ms: u64, device: u8, item: ItemRef) -> Event {
        at(id, wall_ms, device, ALICE, Body::Love(item))
    }

    fn unlove(id: u8, wall_ms: u64, device: u8, item: ItemRef) -> Event {
        at(id, wall_ms, device, ALICE, Body::Unlove(item))
    }

    const LOVED: ItemRef = ItemRef::Content(SONG);
    const PLAYLIST: ItemRef = ItemRef::Document(DocumentId::new([3; 16]));

    #[test]
    fn nothing_is_loved_until_it_is_loved() {
        assert!(!current_love(&EventSet::new(), ALICE, LOVED));
    }

    #[test]
    fn the_latest_clock_wins_for_loves() {
        let set: EventSet = [love(1, 10, 1, LOVED), unlove(2, 20, 1, LOVED)]
            .into_iter()
            .collect();
        assert!(!current_love(&set, ALICE, LOVED));
        let set: EventSet = [love(1, 30, 1, LOVED), unlove(2, 20, 1, LOVED)]
            .into_iter()
            .collect();
        assert!(current_love(&set, ALICE, LOVED));
    }

    #[test]
    fn the_counter_breaks_a_tie_in_wall_time() {
        let mut later = unlove(2, 10, 1, LOVED);
        later.clock = Hlc::new(10, 1);
        let set: EventSet = [love(1, 10, 9, LOVED), later].into_iter().collect();
        assert!(!current_love(&set, ALICE, LOVED));
    }

    #[test]
    fn a_tie_in_clock_goes_to_the_larger_device_id() {
        let set: EventSet = [love(1, 10, 5, LOVED), unlove(2, 10, 4, LOVED)]
            .into_iter()
            .collect();
        assert!(current_love(&set, ALICE, LOVED));
        let set: EventSet = [love(1, 10, 4, LOVED), unlove(2, 10, 5, LOVED)]
            .into_iter()
            .collect();
        assert!(!current_love(&set, ALICE, LOVED));
    }

    #[test]
    fn a_tie_in_clock_and_device_goes_to_the_larger_event_id() {
        let set: EventSet = [love(2, 10, 4, LOVED), unlove(1, 10, 4, LOVED)]
            .into_iter()
            .collect();
        assert!(current_love(&set, ALICE, LOVED));
        let set: EventSet = [love(1, 10, 4, LOVED), unlove(2, 10, 4, LOVED)]
            .into_iter()
            .collect();
        assert!(!current_love(&set, ALICE, LOVED));
    }

    #[test]
    fn loves_are_per_item_and_per_person() {
        let set: EventSet = [
            love(1, 10, 1, PLAYLIST),
            unlove(2, 20, 1, LOVED),
            at(3, 30, 1, BOB, Body::Love(LOVED)),
        ]
        .into_iter()
        .collect();
        assert!(current_love(&set, ALICE, PLAYLIST));
        assert!(!current_love(&set, ALICE, LOVED));
        assert!(current_love(&set, BOB, LOVED));
        assert!(!current_love(&set, BOB, PLAYLIST));
    }

    fn key(text: &str) -> SettingKey {
        SettingKey::new(Untrusted::new(text)).unwrap()
    }

    fn set_to(id: u8, wall_ms: u64, scope: SettingScope, name: &str, value: &[u8]) -> Event {
        let body = Body::Setting(Setting {
            scope,
            key: key(name),
            value: SettingValue::new(Untrusted::new(value)).unwrap(),
        });
        at(id, wall_ms, 1, ALICE, body)
    }

    #[test]
    fn the_latest_setting_wins_per_key_and_scope() {
        let phone = SettingScope::Device(DeviceId::new([6; 16]));
        let set: EventSet = [
            set_to(1, 10, SettingScope::Person, "ui.theme", b"dark"),
            set_to(2, 30, SettingScope::Person, "ui.theme", b"light"),
            set_to(3, 20, SettingScope::Person, "ui.theme", b"sepia"),
            set_to(4, 40, phone, "ui.theme", b"dark"),
            set_to(5, 50, SettingScope::Person, "ui.size", b"large"),
        ]
        .into_iter()
        .collect();
        let value = |scope, name: &str| {
            current_setting(&set, ALICE, scope, &key(name)).map(SettingValue::as_bytes)
        };
        assert_eq!(value(SettingScope::Person, "ui.theme"), Some(&b"light"[..]));
        assert_eq!(value(phone, "ui.theme"), Some(&b"dark"[..]));
        assert_eq!(value(SettingScope::Person, "ui.size"), Some(&b"large"[..]));
        assert_eq!(value(phone, "ui.size"), None);
        let bob = current_setting(&set, BOB, SettingScope::Person, &key("ui.theme"));
        assert_eq!(bob, None);
    }

    #[test]
    fn a_tie_between_settings_goes_to_the_larger_device_then_event_id() {
        let mut low = set_to(2, 10, SettingScope::Person, "a", b"low");
        low.device = DeviceId::new([1; 16]);
        let mut high = set_to(1, 10, SettingScope::Person, "a", b"high");
        high.device = DeviceId::new([2; 16]);
        let set: EventSet = [low, high].into_iter().collect();
        let value = current_setting(&set, ALICE, SettingScope::Person, &key("a"));
        assert_eq!(value.map(SettingValue::as_bytes), Some(&b"high"[..]));
        let set: EventSet = [
            set_to(1, 10, SettingScope::Person, "a", b"first"),
            set_to(2, 10, SettingScope::Person, "a", b"second"),
        ]
        .into_iter()
        .collect();
        let value = current_setting(&set, ALICE, SettingScope::Person, &key("a"));
        assert_eq!(value.map(SettingValue::as_bytes), Some(&b"second"[..]));
    }

    fn place_at(id: u8, wall_ms: u64, device: u8, item: ContentId, offset_ms: u64) -> Event {
        let body = Body::Position(Position {
            item,
            place: Place::Time { offset_ms },
        });
        at(id, wall_ms, device, ALICE, body)
    }

    #[test]
    fn the_latest_position_wins_per_item() {
        let set: EventSet = [
            place_at(1, 10, 1, SONG, 100),
            place_at(2, 30, 1, SONG, 300),
            place_at(3, 20, 1, SONG, 200),
            place_at(4, 5, 1, OTHER_SONG, 50),
            play(5, 40, SONG),
        ]
        .into_iter()
        .collect();
        assert_eq!(
            current_position(&set, ALICE, SONG),
            Some(Place::Time { offset_ms: 300 })
        );
        assert_eq!(
            current_position(&set, ALICE, OTHER_SONG),
            Some(Place::Time { offset_ms: 50 })
        );
        assert_eq!(current_position(&set, BOB, SONG), None);
        assert_eq!(current_position(&EventSet::new(), ALICE, SONG), None);
    }

    #[test]
    fn a_tie_between_positions_goes_to_the_larger_device_then_event_id() {
        let set: EventSet = [place_at(2, 10, 1, SONG, 100), place_at(1, 10, 2, SONG, 200)]
            .into_iter()
            .collect();
        assert_eq!(
            current_position(&set, ALICE, SONG),
            Some(Place::Time { offset_ms: 200 })
        );
        let set: EventSet = [place_at(1, 10, 1, SONG, 100), place_at(2, 10, 1, SONG, 200)]
            .into_iter()
            .collect();
        assert_eq!(
            current_position(&set, ALICE, SONG),
            Some(Place::Time { offset_ms: 200 })
        );
    }

    /// An event of a type this version does not know is kept, merged and
    /// passed through unchanged.
    #[test]
    fn an_unknown_event_is_kept_and_passed_through() {
        let body_type = BodyType {
            tag: 40,
            version: 2,
            skippable: true,
        };
        let unknown = at(
            9,
            10,
            1,
            ALICE,
            Body::Unknown(UnknownBody::new(body_type, vec![1, 2, 3])),
        );
        let a: EventSet = [unknown.clone()].into_iter().collect();
        let b: EventSet = [play(1, 5, SONG)].into_iter().collect();
        let merged = b.merge(&a);
        assert_eq!(
            merged.iter().cloned().collect::<Vec<_>>(),
            [play(1, 5, SONG), unknown]
        );
        let counts = Counts::from([(
            SONG,
            ItemCounts {
                plays: 1,
                skips: 0,
                last_played: Some(Hlc::new(5, 0)),
            },
        )]);
        assert_eq!(derive_counts(&merged, ALICE), counts);
    }

    fn set_of(events: Vec<Event>) -> EventSet {
        events.into_iter().collect()
    }

    fn sets() -> impl Strategy<Value = EventSet> {
        vec(strategies::event(), 0..8).prop_map(set_of)
    }

    /// One event per ID, keeping the larger of two bodies under one ID,
    /// written over the raw list so the oracles below do not go through
    /// [`EventSet`].
    fn unique_by_id(events: &[Event]) -> Vec<Event> {
        let mut by_id = BTreeMap::new();
        for event in events {
            match by_id.entry(event.id) {
                Entry::Vacant(slot) => {
                    slot.insert(event.clone());
                }
                Entry::Occupied(mut slot) => {
                    if event > slot.get() {
                        slot.insert(event.clone());
                    }
                }
            }
        }
        by_id.into_values().collect()
    }

    /// Play and skip counts from the raw list: one play or skip per event
    /// ID, then a saturating count and the latest play clock per item.
    fn counts_from_list(events: &[Event], stream: Stream) -> Counts {
        let mut counts = Counts::new();
        for event in unique_by_id(events) {
            if event.stream != stream {
                continue;
            }
            let (item, played) = match event.body {
                Body::Play(play) => (play.item, true),
                Body::Skip(skip) => (skip.item, false),
                _ => continue,
            };
            let entry = counts.entry(item).or_insert(ItemCounts {
                plays: 0,
                skips: 0,
                last_played: None,
            });
            if played {
                entry.plays = entry.plays.saturating_add(1);
                entry.last_played = entry.last_played.max(Some(event.clock));
            } else {
                entry.skips = entry.skips.saturating_add(1);
            }
        }
        counts
    }

    /// Latest-wins over the raw list: the maximum `(clock, device, id)`
    /// after de-duplicating by ID, among events of `stream` that `pick`
    /// accepts.
    fn latest_from_list<T>(
        events: &[Event],
        stream: Stream,
        mut pick: impl FnMut(&Event) -> Option<T>,
    ) -> Option<T> {
        unique_by_id(events)
            .into_iter()
            .filter(|event| event.stream == stream)
            .filter_map(|event| {
                pick(&event).map(|value| (event.clock, event.device, event.id, value))
            })
            .max_by_key(|(clock, device, id, _)| (*clock, *device, *id))
            .map(|(_, _, _, value)| value)
    }

    fn love_from_list(events: &[Event], stream: Stream, item: ItemRef) -> bool {
        latest_from_list(events, stream, |event| match &event.body {
            Body::Love(loved) if *loved == item => Some(true),
            Body::Unlove(unloved) if *unloved == item => Some(false),
            _ => None,
        }) == Some(true)
    }

    fn setting_from_list(
        events: &[Event],
        stream: Stream,
        scope: SettingScope,
        key: &SettingKey,
    ) -> Option<SettingValue> {
        latest_from_list(events, stream, |event| match &event.body {
            Body::Setting(setting) if setting.scope == scope && setting.key == *key => {
                Some(setting.value.clone())
            }
            _ => None,
        })
    }

    fn position_from_list(events: &[Event], stream: Stream, item: ContentId) -> Option<Place> {
        latest_from_list(events, stream, |event| match &event.body {
            Body::Position(position) if position.item == item => Some(position.place),
            _ => None,
        })
    }

    proptest! {
        /// Merging is commutative, associative and idempotent over event
        /// sets, so replicas that exchange events in any order agree.
        #[test]
        fn merging_is_commutative_associative_and_idempotent(
            a in sets(),
            b in sets(),
            c in sets(),
        ) {
            prop_assert_eq!(a.merge(&b), b.merge(&a));
            prop_assert_eq!(a.merge(&b).merge(&c), a.merge(&b.merge(&c)));
            prop_assert_eq!(a.merge(&a), a.clone());
        }

        /// The derived values match an independent oracle over the raw
        /// list (latest `(clock, device, id)` after de-duplicating by ID),
        /// so they cannot pass merely because two equal sets agree.
        #[test]
        fn derived_values_do_not_depend_on_arrival_order(
            events in vec(strategies::event(), 0..12),
            stream in strategies::stream(),
            item in strategies::content(),
            scope in strategies::setting_scope(),
            key in strategies::setting_key(),
        ) {
            let forwards = set_of(events.clone());
            let backwards = set_of(events.iter().rev().cloned().collect());
            prop_assert_eq!(&forwards, &backwards);
            prop_assert_eq!(derive_counts(&forwards, stream), counts_from_list(&events, stream));
            prop_assert_eq!(
                derive_counts(&backwards, stream),
                counts_from_list(&events, stream)
            );
            let loved = ItemRef::Content(item);
            prop_assert_eq!(
                current_love(&forwards, stream, loved),
                love_from_list(&events, stream, loved)
            );
            prop_assert_eq!(
                current_love(&backwards, stream, loved),
                love_from_list(&events, stream, loved)
            );
            let setting = setting_from_list(&events, stream, scope, &key);
            prop_assert_eq!(
                current_setting(&forwards, stream, scope, &key).cloned(),
                setting.clone()
            );
            prop_assert_eq!(
                current_setting(&backwards, stream, scope, &key).cloned(),
                setting
            );
            prop_assert_eq!(
                current_position(&forwards, stream, item),
                position_from_list(&events, stream, item)
            );
            prop_assert_eq!(
                current_position(&backwards, stream, item),
                position_from_list(&events, stream, item)
            );
        }
    }
}
