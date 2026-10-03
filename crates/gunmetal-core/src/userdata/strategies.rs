//! Generators of events for the property tests of this module.
//!
//! IDs, items and clocks come from small sets, so generated events share
//! them often: the same event ID with two bodies, two loves of one item at
//! one clock, a play and an erasure of it.

use super::event::{
    Body, BodyType, ContentId, DeviceId, DocumentId, DocumentKind, DocumentOp, DocumentRef,
    DocumentSnapshot, Event, EventId, ItemRef, PageOf, Percent, Place, Play, Position, ProfileId,
    Setting, SettingKey, SettingScope, SettingValue, Skip, Stream, TextLocator, UnknownBody,
};
use super::hlc::Hlc;
use crate::untrusted::Untrusted;
use proptest::collection::vec;
use proptest::prelude::*;

/// One of four event IDs.
pub fn event_id() -> impl Strategy<Value = EventId> {
    (0_u8..4).prop_map(|n| EventId::new([n; 16]))
}

/// One of three devices.
pub fn device() -> impl Strategy<Value = DeviceId> {
    (0_u8..3).prop_map(|n| DeviceId::new([n; 16]))
}

/// One of two profiles' streams, or the household's.
pub fn stream() -> impl Strategy<Value = Stream> {
    prop_oneof![
        (0_u8..2).prop_map(|n| Stream::Profile(ProfileId::new([n; 16]))),
        Just(Stream::Household),
    ]
}

/// One of three items.
pub fn content() -> impl Strategy<Value = ContentId> {
    (0_u8..3).prop_map(|n| ContentId::new([n; 32]))
}

/// A clock in a small range, or anywhere.
pub fn clock() -> impl Strategy<Value = Hlc> {
    prop_oneof![
        (0_u64..4, 0_u32..3).prop_map(|(wall, logical)| Hlc::new(wall, logical)),
        (any::<u64>(), any::<u32>()).prop_map(|(wall, logical)| Hlc::new(wall, logical)),
    ]
}

/// A love's or an unlove's item: one of the three items, or a playlist.
fn item() -> impl Strategy<Value = ItemRef> {
    prop_oneof![
        content().prop_map(ItemRef::Content),
        Just(ItemRef::Document(DocumentId::new([9; 16]))),
    ]
}

/// One of two setting keys.
pub fn setting_key() -> impl Strategy<Value = SettingKey> {
    prop_oneof![Just("a"), Just("playback.crossfade")]
        .prop_map(|text| SettingKey::new(Untrusted::new(text)).unwrap())
}

/// A setting's scope: the person, or one of the devices.
pub fn setting_scope() -> impl Strategy<Value = SettingScope> {
    prop_oneof![
        Just(SettingScope::Person),
        device().prop_map(SettingScope::Device),
    ]
}

/// A document of any kind.
fn document() -> impl Strategy<Value = DocumentRef> {
    (
        prop_oneof![
            Just(DocumentKind::Queue),
            Just(DocumentKind::Playlist),
            Just(DocumentKind::HomeLayout),
            Just(DocumentKind::RuleTree),
        ],
        any::<[u8; 16]>(),
    )
        .prop_map(|(kind, id)| DocumentRef {
            kind,
            id: DocumentId::new(id),
        })
}

/// A place in any of its forms, anywhere in its range.
fn place() -> impl Strategy<Value = Place> {
    prop_oneof![
        any::<u64>().prop_map(|offset_ms| Place::Time { offset_ms }),
        (any::<u32>(), 0..=TextLocator::PROGRESSION_END).prop_filter_map(
            "a locator",
            |(resource, progression)| TextLocator::new(resource, progression).map(Place::Text)
        ),
        (1_u32.., any::<u32>()).prop_filter_map("a page of its total", |(page, extra)| {
            PageOf::new(page, page.saturating_add(extra)).map(Place::Page)
        }),
        (0..=Percent::WHOLE).prop_filter_map("a percentage", |basis_points| {
            Percent::new(basis_points).map(Place::Percent)
        }),
    ]
}

/// A type that is none of the known ones, with octets.
fn unknown() -> impl Strategy<Value = UnknownBody> {
    (
        9_u32..1_000,
        any::<u32>(),
        any::<bool>(),
        vec(any::<u8>(), 0..8),
    )
        .prop_map(|(tag, version, skippable, octets)| {
            let body_type = BodyType {
                tag,
                version,
                skippable,
            };
            UnknownBody::new(body_type, octets)
        })
}

/// Any body.
pub fn body() -> impl Strategy<Value = Body> {
    prop_oneof![
        (content(), any::<u64>(), any::<bool>()).prop_map(|(item, position_ms, completed)| {
            Body::Play(Play {
                item,
                position_ms,
                completed,
            })
        }),
        (content(), any::<u64>())
            .prop_map(|(item, position_ms)| Body::Skip(Skip { item, position_ms })),
        item().prop_map(Body::Love),
        item().prop_map(Body::Unlove),
        (setting_scope(), setting_key(), vec(any::<u8>(), 0..8)).prop_map(|(scope, key, value)| {
            Body::Setting(Setting {
                scope,
                key,
                value: SettingValue::new(Untrusted::new(&value)).unwrap(),
            })
        }),
        (document(), any::<u64>(), vec(any::<u8>(), 0..8)).prop_map(
            |(document, based_on, payload)| {
                Body::DocumentOp(DocumentOp {
                    document,
                    based_on,
                    payload,
                })
            }
        ),
        (document(), any::<u64>(), vec(any::<u8>(), 0..8)).prop_map(
            |(document, version, payload)| {
                Body::DocumentSnapshot(DocumentSnapshot {
                    document,
                    version,
                    payload,
                })
            }
        ),
        (content(), place()).prop_map(|(item, place)| Body::Position(Position { item, place })),
        unknown().prop_map(Body::Unknown),
    ]
}

/// Any event.
pub fn event() -> impl Strategy<Value = Event> {
    (event_id(), clock(), device(), stream(), body()).prop_map(
        |(id, clock, device, stream, body)| Event {
            id,
            clock,
            device,
            stream,
            body,
        },
    )
}
