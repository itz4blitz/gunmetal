//! The event envelope and the body types of the first log version (ADR 3,
//! sections 4 and 5).
//!
//! An event names library items only by content identity and other objects
//! only by internal random IDs, never by path, title or public ID (ADR 3,
//! section 14). A play or skip holds exactly what SEC-PRV-002 allows: with
//! the envelope's profile, device and clock, the item, how far it played
//! and whether it finished. A play, skip or position cannot hold an
//! address, a location, a user agent or free text.
//!
//! Every body type has a [`BodyType`]: a tag, the version of that type, and
//! whether an older binary may skip it (ADR 3, section 9). A body whose type
//! this version does not know is kept whole as an [`UnknownBody`], so the
//! types later releases add need no change to the format.
//!
//! Data classes (SEC-PRV-001): every field of [`Play`], [`Skip`] and
//! [`Position`], and the envelope's clock, device and stream, are Activity
//! data. Loves, settings and document bodies are Activity data of the person
//! who authored them.

use crate::untrusted::Untrusted;

/// The random 128-bit ID of one event. A client draws it when it authors
/// the event, so a retry carries the same ID and is stored once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId([u8; 16]);

impl EventId {
    /// The event ID with these random bytes.
    #[must_use]
    pub const fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// The ID's bytes.
    #[must_use]
    pub const fn bytes(self) -> [u8; 16] {
        self.0
    }
}

/// The internal random ID of the device that authored an event, or of the
/// import batch that brought it in (ADR 3, section 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId([u8; 16]);

impl DeviceId {
    /// The device ID with these random bytes.
    #[must_use]
    pub const fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// The ID's bytes.
    #[must_use]
    pub const fn bytes(self) -> [u8; 16] {
        self.0
    }
}

/// The internal random ID of a profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProfileId([u8; 16]);

impl ProfileId {
    /// The profile ID with these random bytes.
    #[must_use]
    pub const fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// The ID's bytes.
    #[must_use]
    pub const fn bytes(self) -> [u8; 16] {
        self.0
    }
}

/// The internal random ID of a versioned document: a queue, a playlist, a
/// Home layout or a rule tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DocumentId([u8; 16]);

impl DocumentId {
    /// The document ID with these random bytes.
    #[must_use]
    pub const fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// The ID's bytes.
    #[must_use]
    pub const fn bytes(self) -> [u8; 16] {
        self.0
    }
}

/// A library item's content identity, as a 32-octet digest.
///
/// The identity rules (WP-077) reduce every identity, a path stem included,
/// to this digest before it reaches an event, so the log can hold no path
/// or title (SEC-PRV-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentId([u8; 32]);

impl ContentId {
    /// The content identity with this digest.
    #[must_use]
    pub const fn new(digest: [u8; 32]) -> Self {
        Self(digest)
    }

    /// The digest.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.0
    }
}

/// The stream an event belongs to: one profile's, or the household's
/// (ADR 3, section 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stream {
    /// One profile's Activity data.
    Profile(ProfileId),
    /// The household's curation, which only administrators write.
    Household,
}

/// One event: the envelope and its body.
///
/// The writer's per-stream sequence number is not part of the event: it is
/// stamped on the record that frames the event (WP-068), because devices
/// author events without one and merging must not depend on it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Event {
    /// The event's random ID.
    pub id: EventId,
    /// When it happened, by the authoring device's hybrid logical clock.
    pub clock: super::hlc::Hlc,
    /// The device, or import batch, that authored it.
    pub device: DeviceId,
    /// The stream it belongs to.
    pub stream: Stream,
    /// What happened.
    pub body: Body,
}

/// What an event says happened.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Body {
    /// An item was played.
    Play(Play),
    /// An item was skipped.
    Skip(Skip),
    /// An item was loved.
    Love(ItemRef),
    /// An item was unloved.
    Unlove(ItemRef),
    /// A preference was set.
    Setting(Setting),
    /// An operation on a versioned document.
    DocumentOp(DocumentOp),
    /// A full snapshot of a versioned document.
    DocumentSnapshot(DocumentSnapshot),
    /// A place in one item (LAT-006).
    Position(Position),
    /// A body of a type this version does not know, kept whole.
    Unknown(UnknownBody),
}

impl Body {
    /// The body's type, version and skippable flag.
    #[must_use]
    pub fn body_type(&self) -> BodyType {
        match self {
            Self::Play(_) => BodyType::PLAY,
            Self::Skip(_) => BodyType::SKIP,
            Self::Love(_) => BodyType::LOVE,
            Self::Unlove(_) => BodyType::UNLOVE,
            Self::Setting(_) => BodyType::SETTING,
            Self::DocumentOp(_) => BodyType::DOCUMENT_OP,
            Self::DocumentSnapshot(_) => BodyType::DOCUMENT_SNAPSHOT,
            Self::Position(_) => BodyType::POSITION,
            Self::Unknown(unknown) => unknown.body_type,
        }
    }

    /// Whether the body is history: a play, a skip or a position, which an
    /// erasure of one event, a range or everything removes (ADR 3,
    /// section 8).
    #[must_use]
    pub const fn is_history(&self) -> bool {
        matches!(self, Self::Play(_) | Self::Skip(_) | Self::Position(_))
    }
}

/// A body's type tag, the version of that type, and whether an older binary
/// may skip it (ADR 3, section 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BodyType {
    /// Which type of body it is.
    pub tag: u32,
    /// The version of that type.
    pub version: u32,
    /// Whether a binary that does not know the type may ignore it without
    /// misreading anything else.
    pub skippable: bool,
}

impl BodyType {
    /// [`Body::Play`].
    pub const PLAY: Self = Self {
        tag: 1,
        version: 1,
        skippable: true,
    };
    /// [`Body::Skip`].
    pub const SKIP: Self = Self {
        tag: 2,
        version: 1,
        skippable: true,
    };
    /// [`Body::Love`].
    pub const LOVE: Self = Self {
        tag: 3,
        version: 1,
        skippable: true,
    };
    /// [`Body::Unlove`].
    pub const UNLOVE: Self = Self {
        tag: 4,
        version: 1,
        skippable: true,
    };
    /// [`Body::Setting`].
    pub const SETTING: Self = Self {
        tag: 5,
        version: 1,
        skippable: true,
    };
    /// [`Body::DocumentOp`]. Not skippable: a reader that skipped one
    /// would misread every later operation on the document.
    pub const DOCUMENT_OP: Self = Self {
        tag: 6,
        version: 1,
        skippable: false,
    };
    /// [`Body::DocumentSnapshot`]. Not skippable, as for operations.
    pub const DOCUMENT_SNAPSHOT: Self = Self {
        tag: 7,
        version: 1,
        skippable: false,
    };
    /// [`Body::Position`].
    pub const POSITION: Self = Self {
        tag: 8,
        version: 1,
        skippable: true,
    };
}

/// A play: the item, how far it played and whether it finished
/// (SEC-PRV-002). The envelope's clock says when.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Play {
    /// The item played.
    pub item: ContentId,
    /// How far it played, in milliseconds from its start.
    pub position_ms: u64,
    /// Whether it played to the end.
    pub completed: bool,
}

/// A skip: the item and how far it had played (SEC-PRV-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Skip {
    /// The item skipped.
    pub item: ContentId,
    /// How far it had played, in milliseconds from its start.
    pub position_ms: u64,
}

/// What a love names: a library item, or a document such as a playlist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ItemRef {
    /// A track, album or artist, by content identity.
    Content(ContentId),
    /// A playlist or another document.
    Document(DocumentId),
}

/// A preference one person set, with a person or a device scope, replaced
/// whole by the next setting of the same key and scope (ADR 3, section 5).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Setting {
    /// Whom the setting applies to.
    pub scope: SettingScope,
    /// Which setting it is.
    pub key: SettingKey,
    /// Its value, encoded by the client that owns the setting.
    pub value: SettingValue,
}

/// Whom a setting applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SettingScope {
    /// The person, on every device.
    Person,
    /// One of the person's devices.
    Device(DeviceId),
}

/// The longest setting key, in octets.
pub const SETTING_KEY_MAX: usize = 64;

/// The longest setting value, in octets.
pub const SETTING_VALUE_MAX: usize = 4_096;

/// A setting's name: 1 to [`SETTING_KEY_MAX`] octets of lower-case ASCII
/// letters, digits, `.`, `_` and `-`, such as `playback.crossfade`. A key
/// is a name the client program fixes, never text a person typed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SettingKey(String);

impl SettingKey {
    /// The key `text`, or `None` when it is empty, longer than
    /// [`SETTING_KEY_MAX`] octets or holds any other character.
    #[must_use]
    pub fn new(text: Untrusted<&str>) -> Option<Self> {
        Self::from_bytes(text.into_inner().as_bytes())
    }

    /// The key these octets spell, under the rules of [`SettingKey::new`].
    pub(crate) fn from_bytes(octets: &[u8]) -> Option<Self> {
        let allowed = |octet: &u8| {
            octet.is_ascii_lowercase() || octet.is_ascii_digit() || b"._-".contains(octet)
        };
        let valid = (1..=SETTING_KEY_MAX).contains(&octets.len()) && octets.iter().all(allowed);
        // Every allowed octet is ASCII, so the conversion cannot fail.
        valid
            .then(|| String::from_utf8(octets.to_vec()).ok())
            .flatten()
            .map(Self)
    }

    /// The key.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A setting's value: at most [`SETTING_VALUE_MAX`] octets.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SettingValue(Vec<u8>);

impl SettingValue {
    /// The value `octets`, or `None` when it is longer than
    /// [`SETTING_VALUE_MAX`].
    #[must_use]
    pub fn new(octets: Untrusted<&[u8]>) -> Option<Self> {
        Self::from_bytes(octets.into_inner())
    }

    /// The value these octets hold, under the rule of
    /// [`SettingValue::new`].
    pub(crate) fn from_bytes(octets: &[u8]) -> Option<Self> {
        (octets.len() <= SETTING_VALUE_MAX).then(|| Self(octets.to_vec()))
    }

    /// The value's octets.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Which kind of versioned document a document body is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DocumentKind {
    /// A profile's queue.
    Queue,
    /// A manual playlist.
    Playlist,
    /// The Home layout and its pins.
    HomeLayout,
    /// A saved rule tree.
    RuleTree,
}

/// A versioned document: its kind and ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DocumentRef {
    /// What kind of document it is.
    pub kind: DocumentKind,
    /// Which document it is.
    pub id: DocumentId,
}

/// One operation on a versioned document. The server orders operations and
/// assigns versions; the document's own module encodes the operation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DocumentOp {
    /// The document.
    pub document: DocumentRef,
    /// The version the operation was built on.
    pub based_on: u64,
    /// The operation, as the document's module encodes it.
    pub payload: Vec<u8>,
}

/// A full snapshot of a versioned document, from which replay starts.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DocumentSnapshot {
    /// The document.
    pub document: DocumentRef,
    /// The version the snapshot holds.
    pub version: u64,
    /// The document, as its module encodes it.
    pub payload: Vec<u8>,
}

/// A typed position: a place in one item (LAT-006; ADR 3, section 5).
///
/// Beyond the envelope's profile, device and clock it holds only the item
/// and the place: never an address, a location or the text around the
/// place. R1 defines the type and writes none; the first media that resume
/// outside a queue write it with no change to the format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    /// The item the place is in.
    pub item: ContentId,
    /// Where in the item.
    pub place: Place,
}

/// A place in one item, in one of the four forms a medium uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Place {
    /// A time offset, for audio and video.
    Time {
        /// Milliseconds from the item's start.
        offset_ms: u64,
    },
    /// A text locator in the Readium style, for text.
    Text(TextLocator),
    /// A page of a total, for paged media.
    Page(PageOf),
    /// A percentage of the whole.
    Percent(Percent),
}

/// A place in a text: the resource it is in, by its index in the
/// publication's reading order, and the progression within that resource,
/// in millionths. It never holds the text around the place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextLocator {
    /// The resource's index in the reading order.
    resource: u32,
    /// How far into the resource, in millionths.
    progression: u32,
}

impl TextLocator {
    /// The progression at the end of a resource: one million millionths.
    pub const PROGRESSION_END: u32 = 1_000_000;

    /// The place `progression` millionths into resource `resource`, or
    /// `None` when `progression` is past [`TextLocator::PROGRESSION_END`].
    #[must_use]
    pub const fn new(resource: u32, progression: u32) -> Option<Self> {
        if progression <= Self::PROGRESSION_END {
            Some(Self {
                resource,
                progression,
            })
        } else {
            None
        }
    }

    /// The resource's index in the reading order.
    #[must_use]
    pub const fn resource(self) -> u32 {
        self.resource
    }

    /// How far into the resource, in millionths.
    #[must_use]
    pub const fn progression(self) -> u32 {
        self.progression
    }
}

/// A page of a total: from 1 to the total.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageOf {
    /// The page, counted from 1.
    page: u32,
    /// How many pages there are.
    total: u32,
}

impl PageOf {
    /// Page `page` of `total`, or `None` unless `page` is from 1 to
    /// `total`.
    #[must_use]
    pub const fn new(page: u32, total: u32) -> Option<Self> {
        if page >= 1 && page <= total {
            Some(Self { page, total })
        } else {
            None
        }
    }

    /// The page, counted from 1.
    #[must_use]
    pub const fn page(self) -> u32 {
        self.page
    }

    /// How many pages there are.
    #[must_use]
    pub const fn total(self) -> u32 {
        self.total
    }
}

/// A percentage, in hundredths of a percent: from 0 to 10,000.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Percent(u16);

impl Percent {
    /// The whole: one hundred percent.
    pub const WHOLE: u16 = 10_000;

    /// `basis_points` hundredths of a percent, or `None` when that is more
    /// than [`Percent::WHOLE`].
    #[must_use]
    pub const fn new(basis_points: u16) -> Option<Self> {
        if basis_points <= Self::WHOLE {
            Some(Self(basis_points))
        } else {
            None
        }
    }

    /// The percentage, in hundredths of a percent.
    #[must_use]
    pub const fn basis_points(self) -> u16 {
        self.0
    }
}

/// A body whose type this version does not know: its type and its octets,
/// exactly as they were read, so it is kept and written back unchanged
/// (ADR 3, section 7).
///
/// It is made only from a type that is not one of the [`BodyType`]
/// constants, by the codec or by [`UnknownBody::opaque`], so a known body is
/// never held as unknown.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnknownBody {
    /// The body's type, version and skippable flag.
    body_type: BodyType,
    /// The body's octets.
    octets: Vec<u8>,
}

impl UnknownBody {
    /// A body of `body_type` holding `octets`. The codec calls this only for
    /// a type that none of the [`BodyType`] constants is.
    pub(crate) const fn new(body_type: BodyType, octets: Vec<u8>) -> Self {
        Self { body_type, octets }
    }

    /// A body of `body_type` holding `octets`, as an import rebuilds an
    /// opaque record from an export, or `None` when `body_type` is one this
    /// version knows, whose body must be read as that type instead.
    #[must_use]
    pub fn opaque(body_type: BodyType, octets: Vec<u8>) -> Option<Self> {
        let known = [
            BodyType::PLAY,
            BodyType::SKIP,
            BodyType::LOVE,
            BodyType::UNLOVE,
            BodyType::SETTING,
            BodyType::DOCUMENT_OP,
            BodyType::DOCUMENT_SNAPSHOT,
            BodyType::POSITION,
        ];
        (!known.contains(&body_type)).then(|| Self::new(body_type, octets))
    }

    /// The body's octets.
    #[must_use]
    pub fn octets(&self) -> &[u8] {
        &self.octets
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_hold_their_bytes() {
        assert_eq!(EventId::new([1; 16]).bytes(), [1; 16]);
        assert_eq!(DeviceId::new([2; 16]).bytes(), [2; 16]);
        assert_eq!(ProfileId::new([3; 16]).bytes(), [3; 16]);
        assert_eq!(DocumentId::new([4; 16]).bytes(), [4; 16]);
        assert_eq!(ContentId::new([5; 32]).digest(), [5; 32]);
    }

    #[test]
    fn each_body_has_its_own_type() {
        let item = ItemRef::Content(ContentId::new([0; 32]));
        let document = DocumentRef {
            kind: DocumentKind::Queue,
            id: DocumentId::new([0; 16]),
        };
        let unknown = BodyType {
            tag: 99,
            version: 3,
            skippable: true,
        };
        let bodies = [
            Body::Play(Play {
                item: ContentId::new([0; 32]),
                position_ms: 0,
                completed: false,
            }),
            Body::Skip(Skip {
                item: ContentId::new([0; 32]),
                position_ms: 0,
            }),
            Body::Love(item),
            Body::Unlove(item),
            Body::Setting(Setting {
                scope: SettingScope::Person,
                key: SettingKey::new(Untrusted::new("a")).unwrap(),
                value: SettingValue::new(Untrusted::new(b"")).unwrap(),
            }),
            Body::DocumentOp(DocumentOp {
                document,
                based_on: 0,
                payload: Vec::new(),
            }),
            Body::DocumentSnapshot(DocumentSnapshot {
                document,
                version: 0,
                payload: Vec::new(),
            }),
            Body::Unknown(UnknownBody::new(unknown, vec![7])),
        ];
        let types: Vec<(u32, u32, bool)> = bodies
            .iter()
            .map(|body| {
                let kind = body.body_type();
                (kind.tag, kind.version, kind.skippable)
            })
            .collect();
        assert_eq!(
            types,
            [
                (1, 1, true),
                (2, 1, true),
                (3, 1, true),
                (4, 1, true),
                (5, 1, true),
                (6, 1, false),
                (7, 1, false),
                (99, 3, true),
            ]
        );
        assert_eq!(
            [
                BodyType::PLAY,
                BodyType::SKIP,
                BodyType::LOVE,
                BodyType::UNLOVE,
                BodyType::SETTING,
                BodyType::DOCUMENT_OP,
                BodyType::DOCUMENT_SNAPSHOT,
                BodyType::POSITION,
            ]
            .map(|kind| (kind.tag, kind.version, kind.skippable)),
            [
                (1, 1, true),
                (2, 1, true),
                (3, 1, true),
                (4, 1, true),
                (5, 1, true),
                (6, 1, false),
                (7, 1, false),
                (8, 1, true),
            ]
        );
        let history: Vec<bool> = bodies.iter().map(Body::is_history).collect();
        assert_eq!(
            history,
            [true, true, false, false, false, false, false, false]
        );
    }

    #[test]
    fn a_position_is_history_of_its_own_type() {
        let at = Body::Position(Position {
            item: ContentId::new([0; 32]),
            place: Place::Time { offset_ms: 5 },
        });
        let kind = at.body_type();
        assert_eq!((kind.tag, kind.version, kind.skippable), (8, 1, true));
        assert!(at.is_history());
    }

    #[test]
    fn a_text_locator_progresses_from_zero_to_one_million() {
        let locator = |resource, progression| {
            TextLocator::new(resource, progression)
                .map(|locator| (locator.resource(), locator.progression()))
        };
        assert_eq!(locator(0, 0), Some((0, 0)));
        assert_eq!(locator(7, 1_000_000), Some((7, 1_000_000)));
        assert_eq!(locator(u32::MAX, 500_000), Some((u32::MAX, 500_000)));
        assert_eq!(locator(7, 1_000_001), None);
        assert_eq!(TextLocator::PROGRESSION_END, 1_000_000);
    }

    #[test]
    fn a_page_is_one_of_its_total() {
        let page = |page, total| PageOf::new(page, total).map(|of| (of.page(), of.total()));
        assert_eq!(page(1, 1), Some((1, 1)));
        assert_eq!(page(3, 200), Some((3, 200)));
        assert_eq!(page(u32::MAX, u32::MAX), Some((u32::MAX, u32::MAX)));
        assert_eq!(page(0, 5), None);
        assert_eq!(page(6, 5), None);
        assert_eq!(page(0, 0), None);
    }

    #[test]
    fn a_percentage_is_in_hundredths_up_to_one_hundred() {
        let percent = |basis_points| Percent::new(basis_points).map(Percent::basis_points);
        assert_eq!(percent(0), Some(0));
        assert_eq!(percent(4_250), Some(4_250));
        assert_eq!(percent(10_000), Some(10_000));
        assert_eq!(percent(10_001), None);
        assert_eq!(percent(u16::MAX), None);
        assert_eq!(Percent::WHOLE, 10_000);
    }

    #[test]
    fn an_unknown_body_keeps_its_octets() {
        let kind = BodyType {
            tag: 40,
            version: 1,
            skippable: false,
        };
        assert_eq!(UnknownBody::new(kind, vec![1, 2, 3]).octets(), [1, 2, 3]);
    }

    /// An import of an export rebuilds the opaque records it holds, but
    /// never holds a known body as unknown.
    #[test]
    fn an_opaque_body_is_only_of_a_type_this_version_does_not_know() {
        let kept = |tag, version, skippable| {
            let body_type = BodyType {
                tag,
                version,
                skippable,
            };
            UnknownBody::opaque(body_type, vec![9, 8]).map(|body| {
                (
                    Body::Unknown(body.clone()).body_type(),
                    body.octets().to_vec(),
                )
            })
        };
        let known = [
            (1, 1, true),
            (2, 1, true),
            (3, 1, true),
            (4, 1, true),
            (5, 1, true),
            (6, 1, false),
            (7, 1, false),
            (8, 1, true),
        ];
        for (tag, version, skippable) in known {
            assert_eq!(kept(tag, version, skippable), None);
        }
        let unknown = [
            (0, 1, true),
            (9, 1, true),
            (40, 1, false),
            (1, 2, true),
            (1, 1, false),
            (6, 1, true),
            (8, 0, true),
        ];
        for (tag, version, skippable) in unknown {
            let body_type = BodyType {
                tag,
                version,
                skippable,
            };
            assert_eq!(kept(tag, version, skippable), Some((body_type, vec![9, 8])));
        }
    }

    #[test]
    fn a_setting_key_is_a_short_lower_case_name() {
        let key =
            |text: &str| SettingKey::new(Untrusted::new(text)).map(|key| key.as_str().to_owned());
        assert_eq!(
            key("playback.cross-fade_2"),
            Some("playback.cross-fade_2".to_owned())
        );
        assert_eq!(key("a"), Some("a".to_owned()));
        let longest = String::from_utf8([b'k'; 64].to_vec()).unwrap();
        assert_eq!(key(&longest), Some(longest.clone()));
        assert_eq!(key(std::str::from_utf8(&[b'k'; 65]).unwrap()), None);
        assert_eq!(key(""), None);
        for refused in [
            "Upper",
            "space here",
            "slash/",
            "colon:",
            "é",
            "tab\t",
            "a+b",
        ] {
            assert_eq!(key(refused), None);
        }
    }

    #[test]
    fn a_setting_value_holds_at_most_four_kibibytes() {
        let octets = [9; 4_097];
        let value = |len: usize| {
            SettingValue::new(Untrusted::new(&octets[..len])).map(|value| value.as_bytes().len())
        };
        assert_eq!(value(0), Some(0));
        assert_eq!(value(4_096), Some(4_096));
        assert_eq!(value(4_097), None);
        assert_eq!(
            SettingValue::new(Untrusted::new(&[1, 2]))
                .unwrap()
                .as_bytes(),
            [1, 2]
        );
    }
}
