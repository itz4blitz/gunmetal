//! The bytes of an event: the payload of one user-log record (ADR 3,
//! section 4).
//!
//! The layout follows postcard's wire conventions. It is written by hand
//! rather than derived, so that the reader charges the step budget, reports
//! the offset of the field that is wrong and refuses every second spelling
//! of a value: unsigned integers as LEB128
//! varints in their shortest form, IDs and digests as their raw octets,
//! flags as one octet `0` or `1`, enum variants as a varint tag before
//! their fields, and octet strings as a varint length before the octets.
//!
//! | Field | Encoding |
//! |---|---|
//! | Event ID | 16 octets |
//! | Clock | wall time (varint), logical counter (varint, at most `u32::MAX`) |
//! | Device ID | 16 octets |
//! | Stream | tag `0` and the profile ID's 16 octets, or tag `1` for the household |
//! | Body type | tag (varint), version (varint), skippable flag |
//! | Body | length (varint) and that many octets |
//!
//! The body's octets, for each known type:
//!
//! | Body | Encoding |
//! |---|---|
//! | Play | content ID (32 octets), position in ms (varint), completed flag |
//! | Skip | content ID (32 octets), position in ms (varint) |
//! | Love, unlove | item: tag `0` and a content ID, or tag `1` and a document ID |
//! | Setting | scope: tag `0` (person), or tag `1` and a device ID; key and value as octet strings |
//! | Document operation | kind tag (`0` queue, `1` playlist, `2` Home layout, `3` rule tree), document ID, version built on (varint), payload as an octet string |
//! | Document snapshot | kind tag, document ID, version (varint), payload as an octet string |
//! | Position | content ID (32 octets), then the place: tag `0` and an offset in ms (varint); tag `1`, the resource's index (varint) and the progression in millionths (varint, at most 1,000,000); tag `2`, the page and the total (varints, 1 ≤ page ≤ total); or tag `3` and hundredths of a percent (varint, at most 10,000) |
//!
//! A body whose tag, version and skippable flag are not exactly one of the
//! known [`BodyType`]s is kept whole as an [`UnknownBody`] and written back
//! octet for octet.
//!
//! Reading is a parse of untrusted input: what comes back from storage may
//! be damaged or planted (boundary TB10). Every input has one reading, so
//! [`decode`] accepts exactly the octets [`encode`] writes: a varint that is
//! longer than it needs to be, a flag other than `0` or `1`, an unknown
//! tag, and octets left over after the last field are all errors. The
//! format is flat, so there is no nesting to count, and every allocation
//! copies octets the input holds.

use super::event::{
    Body, BodyType, ContentId, DeviceId, DocumentId, DocumentKind, DocumentOp, DocumentRef,
    DocumentSnapshot, Event, EventId, ItemRef, PageOf, Percent, Place, Play, Position, ProfileId,
    SETTING_VALUE_MAX, Setting, SettingKey, SettingScope, SettingValue, Skip, Stream, TextLocator,
    UnknownBody,
};
use super::hlc::Hlc;
use crate::parse::{Budget, Cursor, ParseFault};
use crate::problem::{Arg, Describe, Problem, ProblemCode};
use crate::untrusted::Untrusted;

/// Steps [`decode`] may charge per input octet (SEC-MED-007). It charges
/// one step for each field, and every field reads at least one octet, and
/// one step for each octet it copies.
pub const STEPS_PER_BYTE: u64 = 2;

/// Steps [`decode`] may charge on top of [`STEPS_PER_BYTE`]: the one step
/// for a field the input ends before.
pub const FIXED_STEPS: u64 = 1;

/// Writes `event` as the payload of one log record.
#[must_use]
pub fn encode(event: &Event) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&event.id.bytes());
    put_varint(&mut out, event.clock.wall_ms());
    put_varint(&mut out, u64::from(event.clock.logical()));
    out.extend_from_slice(&event.device.bytes());
    match event.stream {
        Stream::Profile(profile) => {
            out.push(0);
            out.extend_from_slice(&profile.bytes());
        }
        Stream::Household => out.push(1),
    }
    let body_type = event.body.body_type();
    put_varint(&mut out, u64::from(body_type.tag));
    put_varint(&mut out, u64::from(body_type.version));
    out.push(u8::from(body_type.skippable));
    put_octets(&mut out, &encode_body(&event.body));
    out
}

/// The octets of `body`.
fn encode_body(body: &Body) -> Vec<u8> {
    let mut out = Vec::new();
    match body {
        Body::Play(play) => {
            out.extend_from_slice(&play.item.digest());
            put_varint(&mut out, play.position_ms);
            out.push(u8::from(play.completed));
        }
        Body::Skip(skip) => {
            out.extend_from_slice(&skip.item.digest());
            put_varint(&mut out, skip.position_ms);
        }
        Body::Love(item) | Body::Unlove(item) => put_item(&mut out, *item),
        Body::Setting(setting) => {
            match setting.scope {
                SettingScope::Person => out.push(0),
                SettingScope::Device(device) => {
                    out.push(1);
                    out.extend_from_slice(&device.bytes());
                }
            }
            put_octets(&mut out, setting.key.as_str().as_bytes());
            put_octets(&mut out, setting.value.as_bytes());
        }
        Body::DocumentOp(op) => {
            put_document(&mut out, op.document);
            put_varint(&mut out, op.based_on);
            put_octets(&mut out, &op.payload);
        }
        Body::DocumentSnapshot(snapshot) => {
            put_document(&mut out, snapshot.document);
            put_varint(&mut out, snapshot.version);
            put_octets(&mut out, &snapshot.payload);
        }
        Body::Position(position) => {
            out.extend_from_slice(&position.item.digest());
            put_place(&mut out, position.place);
        }
        Body::Unknown(unknown) => out.extend_from_slice(unknown.octets()),
    }
    out
}

/// Appends `value` as a LEB128 varint in its shortest form.
fn put_varint(out: &mut Vec<u8>, value: u64) {
    let mut rest = value;
    while rest > 0x7f {
        // The low seven bits, with the bit that says more octets follow.
        out.push(rest.to_le_bytes()[0] | 0x80);
        rest >>= 7;
    }
    out.push(rest.to_le_bytes()[0]);
}

/// Appends `octets` with their length before them.
fn put_octets(out: &mut Vec<u8>, octets: &[u8]) {
    // A slice never holds more than u64::MAX octets.
    put_varint(out, u64::try_from(octets.len()).unwrap_or(u64::MAX));
    out.extend_from_slice(octets);
}

/// Appends what a love names.
fn put_item(out: &mut Vec<u8>, item: ItemRef) {
    match item {
        ItemRef::Content(content) => {
            out.push(0);
            out.extend_from_slice(&content.digest());
        }
        ItemRef::Document(document) => {
            out.push(1);
            out.extend_from_slice(&document.bytes());
        }
    }
}

/// Appends a place: its form's tag, then its numbers.
fn put_place(out: &mut Vec<u8>, place: Place) {
    match place {
        Place::Time { offset_ms } => {
            out.push(0);
            put_varint(out, offset_ms);
        }
        Place::Text(locator) => {
            out.push(1);
            put_varint(out, u64::from(locator.resource()));
            put_varint(out, u64::from(locator.progression()));
        }
        Place::Page(page) => {
            out.push(2);
            put_varint(out, u64::from(page.page()));
            put_varint(out, u64::from(page.total()));
        }
        Place::Percent(percent) => {
            out.push(3);
            put_varint(out, u64::from(percent.basis_points()));
        }
    }
}

/// Appends a document's kind and ID.
fn put_document(out: &mut Vec<u8>, document: DocumentRef) {
    out.push(match document.kind {
        DocumentKind::Queue => 0,
        DocumentKind::Playlist => 1,
        DocumentKind::HomeLayout => 2,
        DocumentKind::RuleTree => 3,
    });
    out.extend_from_slice(&document.id.bytes());
}

/// Reads one event from the payload of one log record, charging `budget`.
///
/// A budget of [`STEPS_PER_BYTE`] × the input's length + [`FIXED_STEPS`]
/// always suffices.
///
/// # Errors
///
/// Returns an [`EventError`] with the offset of the field that is wrong
/// when `input` is not exactly one event as [`encode`] writes it, or when
/// `budget` runs out.
pub fn decode(input: Untrusted<&[u8]>, budget: &mut Budget) -> Result<Event, EventError> {
    let mut reader = Reader {
        cursor: Cursor::new(input.into_inner()),
        budget,
    };
    let event = reader.event()?;
    reader.end()?;
    Ok(event)
}

/// Which field of an event an error is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// A varint: the clock, a body's type, a length, a position or a
    /// version.
    Varint,
    /// A flag: the skippable flag, or whether a play completed.
    Flag,
    /// The stream's tag.
    Stream,
    /// The tag of what a love names.
    Item,
    /// The tag of a setting's scope.
    Scope,
    /// The tag of a document's kind.
    DocumentKind,
    /// A position's place: its form's tag, or numbers out of that form's
    /// range.
    Place,
    /// A setting's key.
    SettingKey,
    /// A setting's value.
    SettingValue,
}

/// Why octets are not an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventError {
    /// The input ended early, or the step budget ran out.
    Fault(ParseFault),
    /// A field holds a value it may not: a varint that is too long or too
    /// large, a flag other than `0` or `1`, an unknown tag, or a setting key
    /// that is not a key.
    Invalid {
        /// Which field.
        field: Field,
        /// Where the field starts, in octets from the start of the input.
        offset: u64,
    },
    /// A setting's value is longer than its cap.
    TooLong {
        /// Which field.
        field: Field,
        /// Its length in octets.
        len: u64,
        /// The longest it may be.
        max: u64,
        /// Where its length starts.
        offset: u64,
    },
    /// Octets are left after the last field of the event or of its body.
    Trailing {
        /// Where the octets that are left start.
        offset: u64,
        /// How many there are.
        count: u64,
    },
}

impl From<ParseFault> for EventError {
    fn from(fault: ParseFault) -> Self {
        Self::Fault(fault)
    }
}

impl Describe for EventError {
    /// A damaged event, with where it is damaged.
    fn problem(&self) -> Problem {
        let offset = match *self {
            Self::Fault(fault) => fault.offset(),
            Self::Invalid { offset, .. }
            | Self::TooLong { offset, .. }
            | Self::Trailing { offset, .. } => offset,
        };
        Problem {
            code: ProblemCode::EventMalformed,
            args: vec![("offset", Arg::Number(offset))],
        }
    }
}

/// A cursor over the input and the budget it charges.
struct Reader<'a, 'b> {
    /// What is left of the input.
    cursor: Cursor<'a>,
    /// The steps left.
    budget: &'b mut Budget,
}

impl<'a> Reader<'a, '_> {
    /// Charges one step for the field that starts here, and returns where
    /// it starts.
    fn field(&mut self) -> Result<u64, EventError> {
        let offset = self.cursor.offset();
        self.budget.charge(1, offset)?;
        Ok(offset)
    }

    /// Reads a fixed number of octets.
    fn array<const N: usize>(&mut self) -> Result<[u8; N], EventError> {
        self.field()?;
        Ok(self.cursor.array()?)
    }

    /// Reads a varint of at most `max`, in its shortest form.
    fn varint(&mut self, max: u64) -> Result<u64, EventError> {
        let offset = self.field()?;
        let invalid = EventError::Invalid {
            field: Field::Varint,
            offset,
        };
        let mut value = 0_u64;
        let mut shift = 0_u32;
        loop {
            let octet = self.cursor.u8()?;
            value |= u64::from(octet & 0x7f).wrapping_shl(shift);
            let last = octet & 0x80 == 0;
            // The tenth octet holds only the top bit of a u64, and a last
            // octet of zero after the first means a longer form than needed.
            if (shift == 63 && octet > 1) || (last && octet == 0 && shift > 0) {
                return Err(invalid);
            }
            if last {
                return if value <= max {
                    Ok(value)
                } else {
                    Err(invalid)
                };
            }
            shift = shift.saturating_add(7);
        }
    }

    /// Reads a one-octet tag of at most `max`.
    fn tag(&mut self, field: Field, max: u8) -> Result<u8, EventError> {
        let offset = self.field()?;
        let tag = self.cursor.u8()?;
        if tag <= max {
            Ok(tag)
        } else {
            Err(EventError::Invalid { field, offset })
        }
    }

    /// Reads a flag.
    fn flag(&mut self) -> Result<bool, EventError> {
        Ok(self.tag(Field::Flag, 1)? == 1)
    }

    /// Reads a varint of at most `u32::MAX`.
    fn varint32(&mut self) -> Result<u32, EventError> {
        let value = self.varint(u64::from(u32::MAX))?;
        // The varint was held to u32::MAX, so the conversion cannot fail.
        Ok(u32::try_from(value).unwrap_or(u32::MAX))
    }

    /// Reads an octet string, charging one step per octet. It borrows the
    /// octets, so a caller checks their length before copying them.
    fn octets(&mut self) -> Result<&'a [u8], EventError> {
        let len = self.varint(u64::MAX)?;
        let offset = self.cursor.offset();
        let octets = self.cursor.take(len)?;
        self.budget.charge(len, offset)?;
        Ok(octets)
    }

    /// Fails unless the input is used up.
    fn end(&self) -> Result<(), EventError> {
        if self.cursor.is_empty() {
            Ok(())
        } else {
            Err(EventError::Trailing {
                offset: self.cursor.offset(),
                count: self.cursor.remaining(),
            })
        }
    }

    /// Reads the envelope and its body.
    fn event(&mut self) -> Result<Event, EventError> {
        let id = EventId::new(self.array()?);
        let wall_ms = self.varint(u64::MAX)?;
        let clock = Hlc::new(wall_ms, self.varint32()?);
        let device = DeviceId::new(self.array()?);
        let stream = match self.tag(Field::Stream, 1)? {
            0 => Stream::Profile(ProfileId::new(self.array()?)),
            _ => Stream::Household,
        };
        let body_type = BodyType {
            tag: self.varint32()?,
            version: self.varint32()?,
            skippable: self.flag()?,
        };
        let len = self.varint(u64::MAX)?;
        let body = Reader {
            cursor: self.cursor.sub(len)?,
            budget: self.budget,
        }
        .body(body_type)?;
        Ok(Event {
            id,
            clock,
            device,
            stream,
            body,
        })
    }

    /// Reads a body of `body_type` from the whole of this reader.
    fn body(mut self, body_type: BodyType) -> Result<Body, EventError> {
        let body = match body_type {
            BodyType::PLAY => Body::Play(Play {
                item: ContentId::new(self.array()?),
                position_ms: self.varint(u64::MAX)?,
                completed: self.flag()?,
            }),
            BodyType::SKIP => Body::Skip(Skip {
                item: ContentId::new(self.array()?),
                position_ms: self.varint(u64::MAX)?,
            }),
            BodyType::LOVE => Body::Love(self.item()?),
            BodyType::UNLOVE => Body::Unlove(self.item()?),
            BodyType::SETTING => Body::Setting(self.setting()?),
            BodyType::DOCUMENT_OP => Body::DocumentOp(DocumentOp {
                document: self.document()?,
                based_on: self.varint(u64::MAX)?,
                payload: self.octets()?.to_vec(),
            }),
            BodyType::DOCUMENT_SNAPSHOT => Body::DocumentSnapshot(DocumentSnapshot {
                document: self.document()?,
                version: self.varint(u64::MAX)?,
                payload: self.octets()?.to_vec(),
            }),
            BodyType::POSITION => Body::Position(Position {
                item: ContentId::new(self.array()?),
                place: self.place()?,
            }),
            _ => {
                // Kept whole: one step per octet copied.
                let octets = self.cursor.rest();
                self.budget
                    .charge(self.cursor.remaining(), self.cursor.offset())?;
                return Ok(Body::Unknown(UnknownBody::new(body_type, octets.to_vec())));
            }
        };
        self.end()?;
        Ok(body)
    }

    /// Reads what a love names.
    fn item(&mut self) -> Result<ItemRef, EventError> {
        Ok(match self.tag(Field::Item, 1)? {
            0 => ItemRef::Content(ContentId::new(self.array()?)),
            _ => ItemRef::Document(DocumentId::new(self.array()?)),
        })
    }

    /// Reads a setting. The key and value are checked against their caps
    /// before they are copied.
    fn setting(&mut self) -> Result<Setting, EventError> {
        let scope = match self.tag(Field::Scope, 1)? {
            0 => SettingScope::Person,
            _ => SettingScope::Device(DeviceId::new(self.array()?)),
        };
        let offset = self.cursor.offset();
        let key = SettingKey::from_bytes(self.octets()?).ok_or(EventError::Invalid {
            field: Field::SettingKey,
            offset,
        })?;
        let offset = self.cursor.offset();
        let octets = self.octets()?;
        let value = SettingValue::from_bytes(octets).ok_or(EventError::TooLong {
            field: Field::SettingValue,
            len: u64::try_from(octets.len()).unwrap_or(u64::MAX),
            max: u64::try_from(SETTING_VALUE_MAX).unwrap_or(u64::MAX),
            offset,
        })?;
        Ok(Setting { scope, key, value })
    }

    /// Reads a place. Numbers out of the range of their form are refused
    /// at the place's start.
    fn place(&mut self) -> Result<Place, EventError> {
        let invalid = EventError::Invalid {
            field: Field::Place,
            offset: self.cursor.offset(),
        };
        let place = match self.tag(Field::Place, 3)? {
            0 => Some(Place::Time {
                offset_ms: self.varint(u64::MAX)?,
            }),
            1 => {
                let resource = self.varint32()?;
                TextLocator::new(resource, self.varint32()?).map(Place::Text)
            }
            2 => {
                let page = self.varint32()?;
                PageOf::new(page, self.varint32()?).map(Place::Page)
            }
            _ => u16::try_from(self.varint32()?)
                .ok()
                .and_then(Percent::new)
                .map(Place::Percent),
        };
        place.ok_or(invalid)
    }

    /// Reads a document's kind and ID.
    fn document(&mut self) -> Result<DocumentRef, EventError> {
        let kind = match self.tag(Field::DocumentKind, 3)? {
            0 => DocumentKind::Queue,
            1 => DocumentKind::Playlist,
            2 => DocumentKind::HomeLayout,
            _ => DocumentKind::RuleTree,
        };
        Ok(DocumentRef {
            kind,
            id: DocumentId::new(self.array()?),
        })
    }
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::super::strategies;
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// The parts of an encoding, joined.
    fn join(parts: &[&[u8]]) -> Vec<u8> {
        parts.concat()
    }

    /// The envelope of every example below up to the body type: event ID
    /// `11…`, clock 1,700,000,000,123 ms and counter 2, device `22…`, and
    /// the stream of profile `33…`.
    fn envelope() -> Vec<u8> {
        join(&[
            &[0x11; 16],
            &[0xfb, 0xd0, 0x95, 0xff, 0xbc, 0x31],
            &[0x02],
            &[0x22; 16],
            &[0x00],
            &[0x33; 16],
        ])
    }

    /// An event with the envelope of [`envelope`] and `body`.
    fn event(body: Body) -> Event {
        Event {
            id: EventId::new([0x11; 16]),
            clock: Hlc::new(1_700_000_000_123, 2),
            device: DeviceId::new([0x22; 16]),
            stream: Stream::Profile(ProfileId::new([0x33; 16])),
            body,
        }
    }

    /// Decodes with a budget that always suffices.
    fn read(bytes: &[u8]) -> Result<Event, EventError> {
        let mut budget = enough(bytes);
        decode(Untrusted::new(bytes), &mut budget)
    }

    /// The budget [`decode`] documents as enough for `bytes`.
    fn enough(bytes: &[u8]) -> Budget {
        let len = u64::try_from(bytes.len()).unwrap();
        Budget::for_input(len, STEPS_PER_BYTE, FIXED_STEPS)
    }

    /// Checks that `event` encodes as exactly `bytes` and that `bytes`
    /// decode as exactly `event`.
    fn both_ways(event: &Event, bytes: &[u8]) {
        assert_eq!(encode(event), bytes);
        assert_eq!(read(bytes), Ok(event.clone()));
    }

    fn played() -> Event {
        event(Body::Play(Play {
            item: ContentId::new([0x44; 32]),
            position_ms: 215_000,
            completed: true,
        }))
    }

    fn played_bytes() -> Vec<u8> {
        join(&[
            &envelope(),
            // Type 1 (play), version 1, skippable.
            &[0x01, 0x01, 0x01],
            // The body's length: 32 + 3 + 1.
            &[36],
            // The item's content identity.
            &[0x44; 32],
            // Played 215,000 ms, to the end.
            &[0xd8, 0x8f, 0x0d],
            &[0x01],
        ])
    }

    /// A play's serialised form holds the event ID, the clock (UTC
    /// milliseconds and a counter), the device ID, the profile ID, the body
    /// type, the item's content identity, the position and the completion
    /// state, and nothing else: no field could carry an address, a
    /// location, a user agent, a title or a path.
    ///
    /// Verifies: SEC-PRV-002
    #[test]
    fn a_play_holds_exactly_the_fields_history_may_hold() {
        both_ways(&played(), &played_bytes());
    }

    /// Verifies: SEC-PRV-002
    #[test]
    fn a_skip_holds_exactly_the_fields_history_may_hold() {
        let skipped = event(Body::Skip(Skip {
            item: ContentId::new([0x44; 32]),
            position_ms: 300,
        }));
        let bytes = join(&[
            &envelope(),
            &[0x02, 0x01, 0x01],
            &[34],
            &[0x44; 32],
            &[0xac, 0x02],
        ]);
        both_ways(&skipped, &bytes);
    }

    #[test]
    fn loves_and_unloves_name_an_item_or_a_document() {
        let loved = event(Body::Love(ItemRef::Content(ContentId::new([0x55; 32]))));
        let bytes = join(&[&envelope(), &[0x03, 0x01, 0x01], &[33, 0x00], &[0x55; 32]]);
        both_ways(&loved, &bytes);
        let unloved = event(Body::Unlove(ItemRef::Document(DocumentId::new([0x66; 16]))));
        let bytes = join(&[&envelope(), &[0x04, 0x01, 0x01], &[17, 0x01], &[0x66; 16]]);
        both_ways(&unloved, &bytes);
    }

    fn setting(scope: SettingScope, value: &[u8]) -> Event {
        event(Body::Setting(Setting {
            scope,
            key: SettingKey::new(Untrusted::new("ui.theme")).unwrap(),
            value: SettingValue::new(Untrusted::new(value)).unwrap(),
        }))
    }

    #[test]
    fn a_setting_holds_its_scope_key_and_value() {
        let bytes = join(&[
            &envelope(),
            &[0x05, 0x01, 0x01],
            &[13, 0x00],
            &[8],
            b"ui.theme",
            &[2, 0xaa, 0xbb],
        ]);
        both_ways(&setting(SettingScope::Person, &[0xaa, 0xbb]), &bytes);
        let device = SettingScope::Device(DeviceId::new([0x77; 16]));
        let bytes = join(&[
            &envelope(),
            &[0x05, 0x01, 0x01],
            &[27, 0x01],
            &[0x77; 16],
            &[8],
            b"ui.theme",
            &[0],
        ]);
        both_ways(&setting(device, &[]), &bytes);
    }

    #[test]
    fn a_setting_value_may_be_four_kibibytes() {
        let value = [0x5a; 4_096];
        let bytes = join(&[
            &envelope(),
            &[0x05, 0x01, 0x01],
            // 1 + 9 + 2 + 4,096 = 4,108 octets.
            &[0x8c, 0x20],
            &[0x00, 8],
            b"ui.theme",
            &[0x80, 0x20],
            &value,
        ]);
        both_ways(&setting(SettingScope::Person, &value), &bytes);
    }

    fn document(kind: DocumentKind) -> DocumentRef {
        DocumentRef {
            kind,
            id: DocumentId::new([0x88; 16]),
        }
    }

    #[test]
    fn document_operations_hold_the_kind_id_version_and_payload() {
        let kinds = [
            (DocumentKind::Queue, 0),
            (DocumentKind::Playlist, 1),
            (DocumentKind::HomeLayout, 2),
            (DocumentKind::RuleTree, 3),
        ];
        for (kind, tag) in kinds {
            let op = event(Body::DocumentOp(DocumentOp {
                document: document(kind),
                based_on: 128,
                payload: vec![1, 2, 3],
            }));
            let bytes = join(&[
                &envelope(),
                &[0x06, 0x01, 0x00],
                &[23, tag],
                &[0x88; 16],
                &[0x80, 0x01],
                &[3, 1, 2, 3],
            ]);
            both_ways(&op, &bytes);
        }
    }

    #[test]
    fn a_document_snapshot_holds_the_kind_id_version_and_payload() {
        let snapshot = event(Body::DocumentSnapshot(DocumentSnapshot {
            document: document(DocumentKind::Playlist),
            version: 127,
            payload: vec![9],
        }));
        let bytes = join(&[
            &envelope(),
            &[0x07, 0x01, 0x00],
            &[20, 0x01],
            &[0x88; 16],
            &[0x7f],
            &[1, 9],
        ]);
        both_ways(&snapshot, &bytes);
    }

    fn at_place(place: Place) -> Event {
        event(Body::Position(Position {
            item: ContentId::new([0x44; 32]),
            place,
        }))
    }

    /// A position holds the item and the place, and nothing else: no
    /// field could carry an address, a location or the text around the
    /// place (LAT-006).
    #[test]
    fn a_position_holds_the_item_and_a_place_in_each_form() {
        let examples = [
            (
                Place::Time { offset_ms: 215_000 },
                // Tag 0, then 215,000 ms.
                vec![36, 0x00, 0xd8, 0x8f, 0x0d],
            ),
            (
                Place::Text(TextLocator::new(3, 1_000_000).unwrap()),
                // Tag 1, resource 3, then 1,000,000 millionths.
                vec![37, 0x01, 0x03, 0xc0, 0x84, 0x3d],
            ),
            (
                Place::Page(PageOf::new(3, 200).unwrap()),
                // Tag 2, page 3, then 200 pages.
                vec![36, 0x02, 0x03, 0xc8, 0x01],
            ),
            (
                Place::Percent(Percent::new(10_000).unwrap()),
                // Tag 3, then 10,000 hundredths of a percent.
                vec![35, 0x03, 0x90, 0x4e],
            ),
        ];
        for (place, tail) in examples {
            let (length, rest) = tail.split_at(1);
            let bytes = join(&[&envelope(), &[0x08, 0x01, 0x01], length, &[0x44; 32], rest]);
            both_ways(&at_place(place), &bytes);
        }
    }

    /// A position body with `place` as it is written after the item.
    fn position_bytes(place: &[u8]) -> Vec<u8> {
        let body = join(&[&[0x44; 32], place]);
        let mut bytes = join(&[&envelope(), &[0x08, 0x01, 0x01]]);
        put_octets(&mut bytes, &body);
        bytes
    }

    #[test]
    fn a_place_out_of_its_range_is_refused_where_the_place_starts() {
        let invalid = Err(EventError::Invalid {
            field: Field::Place,
            offset: 92,
        });
        for place in [
            // A fifth form.
            &[0x04][..],
            // Past the end of the resource.
            &[0x01, 0x03, 0xc1, 0x84, 0x3d],
            // Page 0, and page 6 of 5.
            &[0x02, 0x00, 0x05],
            &[0x02, 0x06, 0x05],
            // 100.01%, and past what 16 bits hold.
            &[0x03, 0x91, 0x4e],
            &[0x03, 0x80, 0x80, 0x04],
        ] {
            assert_eq!(read(&position_bytes(place)), invalid);
        }
        // A resource index past u32 is a bad varint where it starts.
        assert_eq!(
            read(&position_bytes(&[0x01, 0x80, 0x80, 0x80, 0x80, 0x10, 0x00])),
            Err(EventError::Invalid {
                field: Field::Varint,
                offset: 93
            })
        );
    }

    #[test]
    fn the_household_stream_has_no_profile() {
        let mut curation = played();
        curation.stream = Stream::Household;
        let bytes = join(&[
            &[0x11; 16],
            &[0xfb, 0xd0, 0x95, 0xff, 0xbc, 0x31],
            &[0x02],
            &[0x22; 16],
            &[0x01],
            &[0x01, 0x01, 0x01],
            &[36],
            &[0x44; 32],
            &[0xd8, 0x8f, 0x0d],
            &[0x01],
        ]);
        both_ways(&curation, &bytes);
    }

    #[test]
    fn clocks_reach_both_ends_of_their_range() {
        let mut latest = played();
        latest.clock = Hlc::new(u64::MAX, u32::MAX);
        let mut earliest = played();
        earliest.clock = Hlc::ZERO;
        let tail = join(&[&[0x22; 16], &[0x00], &[0x33; 16], &played_bytes()[56..]]);
        let latest_bytes = join(&[
            &[0x11; 16],
            &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01],
            &[0xff, 0xff, 0xff, 0xff, 0x0f],
            &tail,
        ]);
        both_ways(&latest, &latest_bytes);
        let earliest_bytes = join(&[&[0x11; 16], &[0x00], &[0x00], &tail]);
        both_ways(&earliest, &earliest_bytes);
    }

    /// A body of a type this version does not know is kept and written back
    /// octet for octet, as is a known type at another version or with
    /// another skippable flag.
    #[test]
    fn an_unknown_body_is_kept_and_written_back_unchanged() {
        let types = [
            ([0x28, 0x01, 0x00], 40, 1, false),
            ([0x01, 0x02, 0x01], 1, 2, true),
            ([0x01, 0x01, 0x00], 1, 1, false),
            ([0x06, 0x01, 0x01], 6, 1, true),
        ];
        for (prefix, tag, version, skippable) in types {
            let bytes = join(&[&envelope(), &prefix, &[3, 0xde, 0xad, 0x00]]);
            let body_type = BodyType {
                tag,
                version,
                skippable,
            };
            let kept = event(Body::Unknown(UnknownBody::new(
                body_type,
                vec![0xde, 0xad, 0x00],
            )));
            both_ways(&kept, &bytes);
        }
    }

    #[test]
    fn an_empty_unknown_body_is_kept() {
        let bytes = join(&[&envelope(), &[0x80, 0x01, 0x07, 0x01], &[0]]);
        let body_type = BodyType {
            tag: 128,
            version: 7,
            skippable: true,
        };
        both_ways(
            &event(Body::Unknown(UnknownBody::new(body_type, Vec::new()))),
            &bytes,
        );
    }

    /// How the reader meets one field, written out from the tables in this
    /// module's documentation rather than taken from the reader.
    #[derive(Debug, Clone, Copy)]
    enum Shape {
        /// This many octets read at once: an ID or a digest.
        Fixed(u64),
        /// A varint, tag or flag of this many octets, read one at a time.
        Varint(u64),
        /// An octet string: a length of `prefix` octets, then `len` octets.
        Octets { prefix: u64, len: u64 },
        /// An unknown body's octets, copied whole.
        Rest(u64),
    }

    impl Shape {
        /// How many octets the field takes.
        fn size(self) -> u64 {
            match self {
                Self::Fixed(len) | Self::Varint(len) | Self::Rest(len) => len,
                Self::Octets { prefix, len } => prefix + len,
            }
        }

        /// The steps the field charges, and where: one at its start, and
        /// one per octet copied where the octets start.
        fn charges(self, start: u64) -> Vec<(u64, u64)> {
            match self {
                Self::Fixed(_) | Self::Varint(_) => vec![(start, 1)],
                Self::Octets { prefix, len } => vec![(start, 1), (start + prefix, len)],
                Self::Rest(len) => vec![(start, len)],
            }
        }

        /// The fault for input that ends at `cut`, inside this field, which
        /// starts at `start`.
        fn truncated(self, start: u64, cut: u64) -> ParseFault {
            let (offset, needed, available) = match self {
                Self::Fixed(len) | Self::Rest(len) => (start, len, cut - start),
                Self::Varint(_) => (cut, 1, 0),
                Self::Octets { prefix, len } => {
                    if cut < start + prefix {
                        (cut, 1, 0)
                    } else {
                        (start + prefix, len, cut - start - prefix)
                    }
                }
            };
            ParseFault::Truncated {
                offset,
                needed,
                available,
            }
        }
    }

    /// The envelope of [`envelope`], field by field, then the body type.
    const ENVELOPE: [Shape; 9] = [
        Shape::Fixed(16),
        Shape::Varint(6),
        Shape::Varint(1),
        Shape::Fixed(16),
        Shape::Varint(1),
        Shape::Fixed(16),
        Shape::Varint(1),
        Shape::Varint(1),
        Shape::Varint(1),
    ];

    /// Where the body of every sample starts: after the envelope, the body
    /// type and a one-octet length.
    const BODY_START: u64 = 60;

    /// One example: the event, its body type's three octets, its body's
    /// octets and the body's fields.
    type Sample = (Event, [u8; 3], Vec<u8>, Vec<Shape>);

    /// One example of every body and of every form within a body.
    fn samples() -> Vec<Sample> {
        [envelope_samples(), place_samples()].concat()
    }

    /// One example of every known body but a position, and of every form
    /// within those bodies.
    fn envelope_samples() -> Vec<Sample> {
        let content = || ContentId::new([0x44; 32]);
        let queue = DocumentRef {
            kind: DocumentKind::Queue,
            id: DocumentId::new([0x88; 16]),
        };
        let key = || SettingKey::new(Untrusted::new("ui.theme")).unwrap();
        let digest = [0x44; 32];
        vec![
            (
                played(),
                [0x01, 0x01, 0x01],
                join(&[&digest, &[0xd8, 0x8f, 0x0d, 0x01]]),
                vec![Shape::Fixed(32), Shape::Varint(3), Shape::Varint(1)],
            ),
            (
                event(Body::Skip(Skip {
                    item: content(),
                    position_ms: 300,
                })),
                [0x02, 0x01, 0x01],
                join(&[&digest, &[0xac, 0x02]]),
                vec![Shape::Fixed(32), Shape::Varint(2)],
            ),
            (
                event(Body::Love(ItemRef::Content(content()))),
                [0x03, 0x01, 0x01],
                join(&[&[0x00], &digest]),
                vec![Shape::Varint(1), Shape::Fixed(32)],
            ),
            (
                event(Body::Unlove(ItemRef::Document(DocumentId::new([0x66; 16])))),
                [0x04, 0x01, 0x01],
                join(&[&[0x01], &[0x66; 16]]),
                vec![Shape::Varint(1), Shape::Fixed(16)],
            ),
            (
                event(Body::Setting(Setting {
                    scope: SettingScope::Person,
                    key: key(),
                    value: SettingValue::new(Untrusted::new(&[0xaa, 0xbb])).unwrap(),
                })),
                [0x05, 0x01, 0x01],
                join(&[&[0x00, 8], b"ui.theme", &[2, 0xaa, 0xbb]]),
                vec![
                    Shape::Varint(1),
                    Shape::Octets { prefix: 1, len: 8 },
                    Shape::Octets { prefix: 1, len: 2 },
                ],
            ),
            (
                event(Body::Setting(Setting {
                    scope: SettingScope::Device(DeviceId::new([0x77; 16])),
                    key: key(),
                    value: SettingValue::new(Untrusted::new(&[])).unwrap(),
                })),
                [0x05, 0x01, 0x01],
                join(&[&[0x01], &[0x77; 16], &[8], b"ui.theme", &[0]]),
                vec![
                    Shape::Varint(1),
                    Shape::Fixed(16),
                    Shape::Octets { prefix: 1, len: 8 },
                    Shape::Octets { prefix: 1, len: 0 },
                ],
            ),
            (
                event(Body::DocumentOp(DocumentOp {
                    document: queue,
                    based_on: 128,
                    payload: vec![1, 2, 3],
                })),
                [0x06, 0x01, 0x00],
                join(&[&[0x00], &[0x88; 16], &[0x80, 0x01], &[3, 1, 2, 3]]),
                vec![
                    Shape::Varint(1),
                    Shape::Fixed(16),
                    Shape::Varint(2),
                    Shape::Octets { prefix: 1, len: 3 },
                ],
            ),
            (
                event(Body::DocumentSnapshot(DocumentSnapshot {
                    document: queue,
                    version: 127,
                    payload: vec![9],
                })),
                [0x07, 0x01, 0x00],
                join(&[&[0x00], &[0x88; 16], &[0x7f], &[1, 9]]),
                vec![
                    Shape::Varint(1),
                    Shape::Fixed(16),
                    Shape::Varint(1),
                    Shape::Octets { prefix: 1, len: 1 },
                ],
            ),
        ]
    }

    /// One example of a position in each form of place, and one of a body
    /// of an unknown type.
    fn place_samples() -> Vec<Sample> {
        let digest = [0x44; 32];
        let unknown = BodyType {
            tag: 40,
            version: 1,
            skippable: false,
        };
        vec![
            (
                event(Body::Unknown(UnknownBody::new(unknown, vec![1, 2, 3]))),
                [0x28, 0x01, 0x00],
                vec![1, 2, 3],
                vec![Shape::Rest(3)],
            ),
            (
                at_place(Place::Time { offset_ms: 300 }),
                [0x08, 0x01, 0x01],
                join(&[&digest, &[0x00, 0xac, 0x02]]),
                vec![Shape::Fixed(32), Shape::Varint(1), Shape::Varint(2)],
            ),
            (
                at_place(Place::Text(TextLocator::new(3, 128).unwrap())),
                [0x08, 0x01, 0x01],
                join(&[&digest, &[0x01, 0x03, 0x80, 0x01]]),
                vec![
                    Shape::Fixed(32),
                    Shape::Varint(1),
                    Shape::Varint(1),
                    Shape::Varint(2),
                ],
            ),
            (
                at_place(Place::Page(PageOf::new(3, 200).unwrap())),
                [0x08, 0x01, 0x01],
                join(&[&digest, &[0x02, 0x03, 0xc8, 0x01]]),
                vec![
                    Shape::Fixed(32),
                    Shape::Varint(1),
                    Shape::Varint(1),
                    Shape::Varint(2),
                ],
            ),
            (
                at_place(Place::Percent(Percent::new(10_000).unwrap())),
                [0x08, 0x01, 0x01],
                join(&[&digest, &[0x03, 0x90, 0x4e]]),
                vec![Shape::Fixed(32), Shape::Varint(1), Shape::Varint(2)],
            ),
        ]
    }

    /// `body` behind the envelope, its type and a one-octet length.
    fn with_body(body_type: [u8; 3], body: &[u8]) -> Vec<u8> {
        let len = u8::try_from(body.len()).unwrap();
        assert!(len < 0x80);
        join(&[&envelope(), &body_type, &[len], body])
    }

    /// Every field of a sample, with where it starts.
    fn fields(body: &[Shape]) -> Vec<(u64, Shape)> {
        let mut start = 0;
        ENVELOPE
            .iter()
            .chain(&[Shape::Varint(1)])
            .chain(body)
            .map(|shape| {
                let field = (start, *shape);
                start += shape.size();
                field
            })
            .collect()
    }

    #[test]
    fn the_samples_are_what_the_tables_say() {
        for (event, body_type, body, shapes) in samples() {
            both_ways(&event, &with_body(body_type, &body));
            let sizes: u64 = shapes.iter().map(|shape| shape.size()).sum();
            assert_eq!(sizes, u64::try_from(body.len()).unwrap());
        }
        let envelope: u64 = ENVELOPE.iter().map(|shape| shape.size()).sum();
        assert_eq!(envelope + 1, BODY_START);
    }

    /// Input cut anywhere in the envelope is truncated at the field the
    /// cut is in, and input cut in the body at the body's start.
    #[test]
    fn every_cut_short_event_is_truncated_where_it_ends() {
        for (_, body_type, body, _) in samples() {
            let bytes = with_body(body_type, &body);
            let fields = fields(&[]);
            for cut in 0..u64::try_from(bytes.len()).unwrap() {
                let expected = if cut < BODY_START {
                    let (start, shape) = fields
                        .iter()
                        .rev()
                        .find(|(start, _)| *start <= cut)
                        .unwrap();
                    shape.truncated(*start, cut)
                } else {
                    // The body is taken whole, as an unknown body's octets are.
                    Shape::Rest(u64::try_from(body.len()).unwrap()).truncated(BODY_START, cut)
                };
                let len = usize::try_from(cut).unwrap();
                assert_eq!(read(&bytes[..len]), Err(EventError::Fault(expected)));
            }
        }
    }

    /// A body whose length says it ends early is truncated at the body's
    /// field the end is in.
    #[test]
    fn every_cut_short_body_is_truncated_where_it_ends() {
        for (_, body_type, body, shapes) in samples() {
            if matches!(shapes[..], [Shape::Rest(_)]) {
                continue;
            }
            for (start, shape) in fields(&shapes).into_iter().skip(ENVELOPE.len() + 1) {
                for cut in start..start + shape.size() {
                    let len = usize::try_from(cut - BODY_START).unwrap();
                    let expected = shape.truncated(start, cut);
                    assert_eq!(
                        read(&with_body(body_type, &body[..len])),
                        Err(EventError::Fault(expected))
                    );
                }
            }
        }
    }

    /// Every budget short of what an event costs stops the decode at the
    /// field it ran out at, and the exact cost decodes it with nothing
    /// left.
    ///
    /// Verifies: SEC-MED-007
    #[test]
    fn every_budget_short_of_the_cost_stops_where_it_runs_out() {
        for (event, body_type, body, shapes) in samples() {
            let bytes = with_body(body_type, &body);
            let charges: Vec<(u64, u64)> = fields(&shapes)
                .into_iter()
                .flat_map(|(start, shape)| shape.charges(start))
                .collect();
            let cost: u64 = charges.iter().map(|(_, steps)| steps).sum();
            for steps in 0..cost {
                let mut left = steps;
                let (offset, _) = charges
                    .iter()
                    .find(|(_, charge)| {
                        let short = *charge > left;
                        left = left.saturating_sub(*charge);
                        short
                    })
                    .unwrap();
                let mut budget = Budget::for_input(0, 0, steps);
                assert_eq!(
                    decode(Untrusted::new(&bytes), &mut budget),
                    Err(EventError::Fault(ParseFault::BudgetExceeded {
                        offset: *offset
                    }))
                );
            }
            let mut budget = Budget::for_input(0, 0, cost);
            assert_eq!(decode(Untrusted::new(&bytes), &mut budget), Ok(event));
            assert_eq!(budget.remaining(), 0);
        }
    }

    #[test]
    fn a_cut_short_field_reports_where_it_starts_and_what_it_needs() {
        let bytes = played_bytes();
        assert_eq!(
            read(&bytes[..10]),
            Err(EventError::Fault(ParseFault::Truncated {
                offset: 0,
                needed: 16,
                available: 10
            }))
        );
        // The body claims 36 octets and 35 are left.
        assert_eq!(
            read(&bytes[..bytes.len() - 1]),
            Err(EventError::Fault(ParseFault::Truncated {
                offset: 60,
                needed: 36,
                available: 35
            }))
        );
    }

    #[test]
    fn octets_after_the_event_are_refused() {
        let bytes = join(&[&played_bytes(), &[0, 0]]);
        assert_eq!(
            read(&bytes),
            Err(EventError::Trailing {
                offset: 96,
                count: 2
            })
        );
    }

    #[test]
    fn octets_after_a_known_body_are_refused() {
        let bytes = join(&[
            &envelope(),
            &[0x03, 0x01, 0x01],
            &[34, 0x00],
            &[0x55; 32],
            &[0xee],
        ]);
        assert_eq!(
            read(&bytes),
            Err(EventError::Trailing {
                offset: 93,
                count: 1
            })
        );
    }

    /// The envelope with its wall time replaced by `wall`.
    fn with_wall(wall: &[u8]) -> Vec<u8> {
        join(&[&[0x11; 16], wall, &played_bytes()[22..]])
    }

    #[test]
    fn a_varint_longer_than_it_needs_to_be_is_refused() {
        let invalid = Err(EventError::Invalid {
            field: Field::Varint,
            offset: 16,
        });
        assert_eq!(read(&with_wall(&[0x80, 0x00])), invalid);
        assert_eq!(read(&with_wall(&[0xff, 0x00])), invalid);
        let mut zero = played();
        zero.clock = Hlc::new(0, 2);
        assert_eq!(read(&with_wall(&[0x00])), Ok(zero));
        let mut one_twenty_eight = played();
        one_twenty_eight.clock = Hlc::new(128, 2);
        assert_eq!(read(&with_wall(&[0x80, 0x01])), Ok(one_twenty_eight));
    }

    #[test]
    fn a_varint_past_sixty_four_bits_is_refused() {
        let invalid = Err(EventError::Invalid {
            field: Field::Varint,
            offset: 16,
        });
        let nine = [0xff; 9];
        assert_eq!(read(&with_wall(&join(&[&nine, &[0x02]]))), invalid);
        assert_eq!(read(&with_wall(&join(&[&nine, &[0x81, 0x00]]))), invalid);
        let mut latest = played();
        latest.clock = Hlc::new(u64::MAX, 2);
        assert_eq!(read(&with_wall(&join(&[&nine, &[0x01]]))), Ok(latest));
        let mut top_bit = played();
        top_bit.clock = Hlc::new(1 << 63, 2);
        let top = join(&[&[0x80; 9], &[0x01]]);
        assert_eq!(read(&with_wall(&top)), Ok(top_bit));
    }

    #[test]
    fn a_counter_past_u32_is_refused() {
        let bytes = join(&[
            &[0x11; 16],
            &[0x00],
            &[0x80, 0x80, 0x80, 0x80, 0x10],
            &played_bytes()[23..],
        ]);
        assert_eq!(
            read(&bytes),
            Err(EventError::Invalid {
                field: Field::Varint,
                offset: 17
            })
        );
    }

    #[test]
    fn a_body_type_past_u32_is_refused() {
        let at = |prefix: &[u8]| join(&[&envelope(), prefix, &[0]]);
        let invalid = |offset| {
            Err(EventError::Invalid {
                field: Field::Varint,
                offset,
            })
        };
        assert_eq!(
            read(&at(&[0x80, 0x80, 0x80, 0x80, 0x10, 0x01, 0x00])),
            invalid(56)
        );
        assert_eq!(
            read(&at(&[0x01, 0x80, 0x80, 0x80, 0x80, 0x10, 0x00])),
            invalid(57)
        );
        let body_type = BodyType {
            tag: u32::MAX,
            version: u32::MAX,
            skippable: false,
        };
        let largest = join(&[
            &envelope(),
            &[
                0xff, 0xff, 0xff, 0xff, 0x0f, 0xff, 0xff, 0xff, 0xff, 0x0f, 0x00,
            ],
            &[0],
        ]);
        both_ways(
            &event(Body::Unknown(UnknownBody::new(body_type, Vec::new()))),
            &largest,
        );
    }

    #[test]
    fn flags_are_zero_or_one() {
        let bytes = join(&[&envelope(), &[0x01, 0x01, 0x02], &[0]]);
        assert_eq!(
            read(&bytes),
            Err(EventError::Invalid {
                field: Field::Flag,
                offset: 58
            })
        );
        let mut played = played_bytes();
        let last = played.len() - 1;
        played[last] = 2;
        assert_eq!(
            read(&played),
            Err(EventError::Invalid {
                field: Field::Flag,
                offset: 95
            })
        );
    }

    #[test]
    fn unknown_tags_are_refused() {
        let mut stream = played_bytes();
        stream[39] = 2;
        assert_eq!(
            read(&stream),
            Err(EventError::Invalid {
                field: Field::Stream,
                offset: 39
            })
        );
        let item = join(&[&envelope(), &[0x03, 0x01, 0x01], &[1, 0x02]]);
        assert_eq!(
            read(&item),
            Err(EventError::Invalid {
                field: Field::Item,
                offset: 60
            })
        );
        let scope = join(&[&envelope(), &[0x05, 0x01, 0x01], &[1, 0x02]]);
        assert_eq!(
            read(&scope),
            Err(EventError::Invalid {
                field: Field::Scope,
                offset: 60
            })
        );
        let kind = join(&[&envelope(), &[0x07, 0x01, 0x00], &[1, 0x04]]);
        assert_eq!(
            read(&kind),
            Err(EventError::Invalid {
                field: Field::DocumentKind,
                offset: 60
            })
        );
    }

    /// A setting body with `key` and `value` as they are written.
    fn setting_bytes(key: &[u8], value: &[u8]) -> Vec<u8> {
        let mut body = vec![0x00];
        put_octets(&mut body, key);
        put_octets(&mut body, value);
        let mut bytes = join(&[&envelope(), &[0x05, 0x01, 0x01]]);
        put_octets(&mut bytes, &body);
        bytes
    }

    #[test]
    fn a_setting_key_must_be_a_key() {
        for key in [&b""[..], b"Theme", b"ui theme", &[b'k'; 65]] {
            assert_eq!(
                read(&setting_bytes(key, b"")),
                Err(EventError::Invalid {
                    field: Field::SettingKey,
                    offset: 61
                })
            );
        }
    }

    /// The cap applies to the value as decoded, before it is copied.
    #[test]
    fn a_setting_value_past_its_cap_is_refused() {
        assert_eq!(
            read(&setting_bytes(b"k", &[0; 4_097])),
            Err(EventError::TooLong {
                field: Field::SettingValue,
                len: 4_097,
                max: 4_096,
                offset: 64
            })
        );
    }

    /// The play example costs one step for each of its 13 fields, and one
    /// for each octet it copies, which it has none of.
    #[test]
    fn decoding_a_play_costs_one_step_per_field() {
        let bytes = played_bytes();
        let mut budget = Budget::for_input(0, 0, 100);
        assert_eq!(decode(Untrusted::new(&bytes), &mut budget), Ok(played()));
        assert_eq!(budget.remaining(), 87);
    }

    /// Copying octets costs one step per octet.
    #[test]
    fn copied_octets_cost_one_step_each() {
        let unknown = join(&[&envelope(), &[0x28, 0x01, 0x00], &[3, 1, 2, 3]]);
        let mut budget = Budget::for_input(0, 0, 100);
        let body_type = BodyType {
            tag: 40,
            version: 1,
            skippable: false,
        };
        assert_eq!(
            decode(Untrusted::new(&unknown), &mut budget),
            Ok(event(Body::Unknown(UnknownBody::new(
                body_type,
                vec![1, 2, 3]
            ))))
        );
        // Ten fields and three octets.
        assert_eq!(budget.remaining(), 87);
        let op = join(&[
            &envelope(),
            &[0x06, 0x01, 0x00],
            &[22, 0x00],
            &[0x88; 16],
            &[0x05],
            &[3, 1, 2, 3],
        ]);
        let mut budget = Budget::for_input(0, 0, 100);
        let queue = DocumentRef {
            kind: DocumentKind::Queue,
            id: DocumentId::new([0x88; 16]),
        };
        assert_eq!(
            decode(Untrusted::new(&op), &mut budget),
            Ok(event(Body::DocumentOp(DocumentOp {
                document: queue,
                based_on: 5,
                payload: vec![1, 2, 3],
            })))
        );
        // Fourteen fields and three octets.
        assert_eq!(budget.remaining(), 83);
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn a_spent_budget_stops_the_decode_where_it_ran_out() {
        let bytes = played_bytes();
        let mut budget = Budget::for_input(0, 0, 12);
        assert_eq!(
            decode(Untrusted::new(&bytes), &mut budget),
            Err(EventError::Fault(ParseFault::BudgetExceeded { offset: 95 }))
        );
        let unknown = join(&[&envelope(), &[0x28, 0x01, 0x00], &[3, 1, 2, 3]]);
        let mut budget = Budget::for_input(0, 0, 12);
        assert_eq!(
            decode(Untrusted::new(&unknown), &mut budget),
            Err(EventError::Fault(ParseFault::BudgetExceeded { offset: 60 }))
        );
        let op = join(&[
            &envelope(),
            &[0x06, 0x01, 0x00],
            &[22, 0x00],
            &[0x88; 16],
            &[0x05],
            &[3, 1, 2, 3],
        ]);
        let mut budget = Budget::for_input(0, 0, 16);
        assert_eq!(
            decode(Untrusted::new(&op), &mut budget),
            Err(EventError::Fault(ParseFault::BudgetExceeded { offset: 79 }))
        );
    }

    #[test]
    fn describes_every_error_as_a_damaged_event_with_its_offset() {
        let errors = [
            EventError::Fault(ParseFault::BudgetExceeded { offset: 3 }),
            EventError::Invalid {
                field: Field::Flag,
                offset: 5,
            },
            EventError::TooLong {
                field: Field::SettingValue,
                len: 9,
                max: 8,
                offset: 7,
            },
            EventError::Trailing {
                offset: 11,
                count: 1,
            },
        ];
        let problems: Vec<Problem> = errors.iter().map(Describe::problem).collect();
        let expected: Vec<Problem> = [3, 5, 7, 11]
            .into_iter()
            .map(|offset| Problem {
                code: ProblemCode::EventMalformed,
                args: vec![("offset", Arg::Number(offset))],
            })
            .collect();
        assert_eq!(problems, expected);
    }

    /// Runs `work` on a fresh thread with the 256 KiB stack SEC-MED-001
    /// names, so a decode that recursed too deeply would fail here.
    fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(262_144)
            .spawn(work)
            .unwrap()
            .join()
            .unwrap()
    }

    /// Whether `bytes` decode, with the budget [`decode`] documents, as
    /// an event that encodes as exactly `bytes` again, or are refused for
    /// a reason other than the budget.
    fn one_reading(bytes: Vec<u8>) -> bool {
        on_small_stack(move || match read(&bytes) {
            Ok(event) => encode(&event) == bytes,
            Err(error) => !matches!(error, EventError::Fault(ParseFault::BudgetExceeded { .. })),
        })
    }

    /// An encoding with some octets changed, some cut and some added.
    fn damaged() -> impl Strategy<Value = Vec<u8>> {
        (
            strategies::event(),
            vec((any::<prop::sample::Index>(), any::<u8>()), 0..3),
            any::<prop::sample::Index>(),
            vec(any::<u8>(), 0..3),
        )
            .prop_map(|(event, changes, cut, extra)| {
                let mut bytes = encode(&event);
                for (at, octet) in changes {
                    let at = at.index(bytes.len());
                    bytes[at] = octet;
                }
                bytes.truncate(cut.index(bytes.len().saturating_add(1)));
                bytes.extend(extra);
                bytes
            })
    }

    proptest! {
        #[test]
        fn every_event_reads_back_as_itself(event in strategies::event()) {
            let bytes = encode(&event);
            prop_assert_eq!(read(&bytes), Ok(event));
        }

        /// Verifies: SEC-MED-001, SEC-MED-007
        #[test]
        fn any_input_has_one_reading_within_the_budget(
            bytes in prop_oneof![
                vec(any::<u8>(), 0..128),
                damaged(),
                strategies::event().prop_map(|event| encode(&event)),
            ],
        ) {
            prop_assert!(one_reading(bytes));
        }
    }
}
