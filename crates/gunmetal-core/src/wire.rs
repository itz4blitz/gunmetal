//! The wire codec: frames that carry protocol types between the server, its
//! workers and its clients, and the rule for which protocol version two
//! peers speak.
//!
//! # Frames
//!
//! A frame is a little-endian length, then the protocol version, the frame
//! kind and the payload:
//!
//! ```text
//! length:  u32 LE   octets after this field: 3 + payload, at most MAX_FRAME
//! version: u16 LE   the protocol version the payload is written in
//! kind:    u8       a FrameKind octet
//! payload: [u8; length - 3]
//! ```
//!
//! [`read_frame`] refuses a length over the cap before it waits for a single
//! payload octet, so a peer that declares four gigabytes gets an error, not a
//! buffer (SEC-MED-023, SEC-TM-032). A length smaller than the version and
//! kind it must cover is an error too, so every frame read moves at least
//! seven octets forward (SEC-MED-008).
//!
//! # Payloads
//!
//! A payload is one protocol type in `postcard`'s encoding (owner decision
//! D-02). Postcard is not self-describing: it builds only the type the
//! caller names and has no way to name another, so no frame can construct a
//! type the receiver did not ask for (SEC-HIS-035). [`decode`] refuses
//! octets left over after that type, which is how a field added by a newer
//! peer shows up.
//!
//! [`decode`] runs postcard through a guard that sees every value before
//! the type does. Each value spends one step of the caller's [`Budget`]
//! (SEC-MED-007) and goes one level deeper against the container depth in
//! [`Limits`] (SEC-MED-005); every sequence and map counts its entries
//! against [`LimitKind::Children`]; and every string is held to
//! [`LimitKind::LongText`] once decoded (SEC-MED-006). The count a sequence
//! or map declares is never passed on to the type, so a type cannot size an
//! allocation from it; its entries are counted as they arrive. Octet strings are
//! bounded only by the frame. Carry octets with `serialize_bytes`, not as a
//! sequence of `u8`, whose entries count as children.
//!
//! A decoded value is still untrusted: the receiver revalidates every
//! string, number and enum through its own typed constructors before it
//! stores anything (SEC-MED-023).
//!
//! # Steps (SEC-MED-007)
//!
//! [`read_frame`] spends one step per frame, and a frame is at least seven
//! octets. [`decode`] spends one step per value. Most values take at least
//! one octet; units, unit variants and the shells of structs, tuples and
//! newtypes take none, and they nest at most as deep as the depth limit.
//! [`payload_budget`] therefore allows [`STEPS_PER_OCTET`] steps per octet
//! plus [`FIXED_STEPS`]. A payload made of zero-octet values, such as a
//! long sequence of units, runs out of steps instead of running forever.
//!
//! # Versions
//!
//! A server speaks its current protocol version and the one before it
//! ([`server_versions`]). [`negotiate`] picks the newest version both sides
//! speak, or says which side has to update.

use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

use postcard::de_flavors::Flavor;
use serde::de::{
    self, DeserializeSeed, Deserializer, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor,
};
use serde::{Deserialize, Serialize};

use crate::parse::{Budget, Cursor, Depth, LimitKind, Limits, ParseFault};

/// What postcard reports in [`WireError::Malformed`] and
/// [`WireError::Unencodable`], named here so that a caller can match on it
/// without depending on postcard itself.
pub use postcard::Error as PostcardError;

/// The most octets a frame may hold after its length field: 32 MiB, the
/// IPC frame cap (SEC-MED-023).
pub const MAX_FRAME: u32 = 33_554_432;

/// Steps [`payload_budget`] allows per payload octet (k in SEC-MED-007).
pub const STEPS_PER_OCTET: u64 = 4;

/// Steps [`payload_budget`] allows on top of [`STEPS_PER_OCTET`] (c in
/// SEC-MED-007): room for zero-octet shells as deep as the depth ceiling,
/// twice over.
pub const FIXED_STEPS: u64 = 64;

/// A protocol version. Versions count up from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProtocolVersion(pub u16);

impl ProtocolVersion {
    /// The version this build writes.
    pub const CURRENT: Self = Self(1);

    /// The version before this one, if there is one.
    #[must_use]
    pub const fn previous(self) -> Option<Self> {
        match self.0.checked_sub(1) {
            Some(0) | None => None,
            Some(earlier) => Some(Self(earlier)),
        }
    }
}

/// What a frame carries, which says how to read its payload.
///
/// Adding a kind changes an interface every peer uses, so it is its own
/// package under the merge protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameKind {
    /// Asks the peer to do something: a job for a worker, a sync request.
    Request,
    /// The whole answer to a request.
    Response,
    /// One part of an answer that arrives as a stream of frames, such as a
    /// sync snapshot or packaged audio.
    Part,
    /// Closes a stream of parts.
    End,
    /// The peer refused a request; the payload says why.
    Refusal,
}

impl FrameKind {
    /// The kind written as `octet`, if it is one.
    #[must_use]
    pub const fn from_octet(octet: u8) -> Option<Self> {
        match octet {
            1 => Some(Self::Request),
            2 => Some(Self::Response),
            3 => Some(Self::Part),
            4 => Some(Self::End),
            5 => Some(Self::Refusal),
            _ => None,
        }
    }

    /// The octet this kind is written as.
    #[must_use]
    pub const fn octet(self) -> u8 {
        match self {
            Self::Request => 1,
            Self::Response => 2,
            Self::Part => 3,
            Self::End => 4,
            Self::Refusal => 5,
        }
    }
}

/// One frame [`read_frame`] found at the start of its input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame<'a> {
    /// The protocol version the payload is written in.
    pub version: ProtocolVersion,
    /// What the frame carries.
    pub kind: FrameKind,
    /// The payload, borrowed from the input.
    pub payload: &'a [u8],
    /// Octets the whole frame took, length field included; the next frame
    /// starts here.
    pub len: usize,
}

/// Why a frame could not be written or read, or a payload decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireError {
    /// The input ended inside the frame, the step budget ran out, or a
    /// decoded value went past a depth, count or length limit.
    Fault(ParseFault),
    /// The frame's length is smaller than the version and kind it must
    /// cover (SEC-MED-008).
    TooShort {
        /// The length the frame declared.
        len: u32,
    },
    /// The frame, or the payload given to [`decode`], is longer than the
    /// cap (SEC-MED-023).
    TooLarge {
        /// Octets after the length field, as declared or as it would be.
        len: u64,
        /// The cap that applied.
        max: u32,
    },
    /// The frame's kind octet names no [`FrameKind`].
    UnknownKind {
        /// The octet.
        kind: u8,
    },
    /// The payload is not an encoding of the type asked for.
    Malformed {
        /// Payload octets read when postcard stopped.
        offset: u64,
        /// What postcard reported.
        reason: postcard::Error,
    },
    /// Octets were left after the value: the peer wrote a type with more
    /// fields than the one asked for (SEC-HIS-035).
    Trailing {
        /// Where the octets left over start in the payload.
        offset: u64,
        /// How many there are.
        len: u64,
    },
    /// The value could not be written in postcard's encoding.
    Unencodable {
        /// What postcard reported.
        reason: postcard::Error,
    },
}

impl From<ParseFault> for WireError {
    fn from(fault: ParseFault) -> Self {
        Self::Fault(fault)
    }
}

/// Why two peers share no protocol version, and which of them is behind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Upgrade {
    /// The client must update: it offered no version, or only versions
    /// older than the newest the server speaks.
    Client {
        /// The newest version the server speaks, if it speaks any.
        server_newest: Option<ProtocolVersion>,
    },
    /// The server must update: the client's newest version is newer than
    /// every version the server speaks.
    Server {
        /// The newest version the client offered.
        client_newest: ProtocolVersion,
    },
}

/// The versions a server whose current version is `current` speaks:
/// `current`, then the one before it if there is one.
#[must_use]
pub fn server_versions(current: ProtocolVersion) -> Vec<ProtocolVersion> {
    std::iter::once(current).chain(current.previous()).collect()
}

/// The newest version both `client` and `server` speak.
///
/// # Errors
///
/// Returns the [`Upgrade`] that says which side is behind when they share
/// none.
pub fn negotiate(
    client: &[ProtocolVersion],
    server: &[ProtocolVersion],
) -> Result<ProtocolVersion, Upgrade> {
    let shared = client
        .iter()
        .copied()
        .filter(|version| server.contains(version))
        .max();
    if let Some(version) = shared {
        return Ok(version);
    }
    // With no version shared, the newest version either side speaks belongs
    // to exactly one of them, and the other side is the one behind.
    match client.iter().chain(server).copied().max() {
        Some(newest) if !server.contains(&newest) => Err(Upgrade::Server {
            client_newest: newest,
        }),
        _ => Err(Upgrade::Client {
            server_newest: server.iter().copied().max(),
        }),
    }
}

/// The step budget [`decode`] should run under for `payload`.
#[must_use]
pub fn payload_budget(payload: &[u8]) -> Budget {
    Budget::for_input(
        u64::try_from(payload.len()).unwrap_or(u64::MAX),
        STEPS_PER_OCTET,
        FIXED_STEPS,
    )
}

/// Writes `value` as one frame of `kind` in protocol `version`.
///
/// # Errors
///
/// [`WireError::Unencodable`] when postcard cannot write `value`, and
/// [`WireError::TooLarge`] when the frame would be longer than
/// [`MAX_FRAME`].
pub fn write_frame<T: Serialize + ?Sized>(
    version: ProtocolVersion,
    kind: FrameKind,
    value: &T,
) -> Result<Vec<u8>, WireError> {
    postcard::to_extend(value, Vec::new())
        .map_err(|reason| WireError::Unencodable { reason })
        .and_then(|payload| seal(version, kind, &payload))
}

/// Puts an encoded `payload` in a frame of `kind` in protocol `version`.
fn seal(version: ProtocolVersion, kind: FrameKind, payload: &[u8]) -> Result<Vec<u8>, WireError> {
    let len = u32::try_from(payload.len())
        .ok()
        .and_then(|len| len.checked_add(3))
        .filter(|&len| len <= MAX_FRAME)
        .ok_or_else(|| WireError::TooLarge {
            len: u64::try_from(payload.len())
                .unwrap_or(u64::MAX)
                .saturating_add(3),
            max: MAX_FRAME,
        })?;
    let mut frame = Vec::new();
    frame.extend(len.to_le_bytes());
    frame.extend(version.0.to_le_bytes());
    frame.push(kind.octet());
    frame.extend(payload);
    Ok(frame)
}

/// Reads the frame at the start of `input`, refusing one longer than `max`
/// octets after its length field, or than [`MAX_FRAME`] whatever `max` is.
///
/// The payload is borrowed, not copied. The input may hold more after the
/// frame; [`Frame::len`] says where the next one starts.
///
/// # Errors
///
/// [`WireError::Fault`] with [`ParseFault::BudgetExceeded`] when `budget`
/// is spent and with [`ParseFault::Truncated`] when the input ends inside
/// the frame, so a reader knows how many octets to wait for;
/// [`WireError::TooLarge`] for a length over the cap, before any payload
/// arrives; [`WireError::TooShort`] for a length under 3; and
/// [`WireError::UnknownKind`].
pub fn read_frame<'a>(
    input: &'a [u8],
    max: u32,
    budget: &mut Budget,
) -> Result<Frame<'a>, WireError> {
    budget.charge(1, 0)?;
    let max = max.min(MAX_FRAME);
    let mut cursor = Cursor::new(input);
    let len = cursor.u32_le()?;
    if len > max {
        return Err(WireError::TooLarge {
            len: u64::from(len),
            max,
        });
    }
    let body = cursor.take(u64::from(len))?;
    let [low, high, kind, payload @ ..] = body else {
        return Err(WireError::TooShort { len });
    };
    let kind = FrameKind::from_octet(*kind).ok_or(WireError::UnknownKind { kind: *kind })?;
    Ok(Frame {
        version: ProtocolVersion(u16::from_le_bytes([*low, *high])),
        kind,
        payload,
        len: input.len().saturating_sub(cursor.rest().len()),
    })
}

/// Decodes `payload` as one `T`, under `limits` and `budget`.
///
/// # Errors
///
/// [`WireError::TooLarge`] for a payload longer than [`MAX_FRAME`];
/// [`WireError::Fault`] when the budget runs out or a value goes past a
/// depth, count or length limit, even if `T` ignored the error;
/// [`WireError::Malformed`] when the octets are not a `T`; and
/// [`WireError::Trailing`] when octets are left after it.
pub fn decode<'de, T: Deserialize<'de>>(
    payload: &'de [u8],
    limits: &Limits,
    budget: &mut Budget,
) -> Result<T, WireError> {
    if !u32::try_from(payload.len()).is_ok_and(|len| len <= MAX_FRAME) {
        return Err(WireError::TooLarge {
            len: u64::try_from(payload.len()).unwrap_or(u64::MAX),
            max: MAX_FRAME,
        });
    }
    let at = Rc::new(Cell::new(0));
    let mut postcard = postcard::Deserializer::from_flavor(Counted {
        bytes: payload,
        at: Rc::clone(&at),
    });
    let mut watch = Watch {
        limits,
        budget,
        depth: Depth::CONTAINER_ROOT,
        at,
        fault: None,
    };
    let decoded = T::deserialize(Guard {
        inner: &mut postcard,
        watch: &mut watch,
    });
    if let Some(fault) = watch.fault {
        return Err(WireError::Fault(fault));
    }
    let offset = watch.offset();
    let value = decoded.map_err(|reason| WireError::Malformed { offset, reason })?;
    let left = postcard.finalize().unwrap_or(0);
    if left > 0 {
        return Err(WireError::Trailing {
            offset,
            len: u64::try_from(left).unwrap_or(u64::MAX),
        });
    }
    Ok(value)
}

/// Postcard's input: the payload, and how many of its octets postcard has
/// taken, which the guard reads for the offsets in its faults.
struct Counted<'de> {
    /// The payload.
    bytes: &'de [u8],
    /// Octets taken so far, shared with the [`Watch`].
    at: Rc<Cell<usize>>,
}

impl<'de> Flavor<'de> for Counted<'de> {
    /// Octets left over.
    type Remainder = usize;
    type Source = &'de [u8];

    fn pop(&mut self) -> postcard::Result<u8> {
        let at = self.at.get();
        let octet = self
            .bytes
            .get(at)
            .copied()
            .ok_or(postcard::Error::DeserializeUnexpectedEnd)?;
        self.at.set(at.saturating_add(1));
        Ok(octet)
    }

    fn try_take_n(&mut self, ct: usize) -> postcard::Result<&'de [u8]> {
        let at = self.at.get();
        let taken = self
            .bytes
            .get(at..)
            .and_then(|rest| rest.get(..ct))
            .ok_or(postcard::Error::DeserializeUnexpectedEnd)?;
        self.at.set(at.saturating_add(taken.len()));
        Ok(taken)
    }

    fn finalize(self) -> postcard::Result<usize> {
        Ok(self.bytes.len().saturating_sub(self.at.get()))
    }
}

/// What one decode has spent and where it is, shared by every layer of the
/// guard.
struct Watch<'w> {
    /// The limits the decode runs under.
    limits: &'w Limits,
    /// The steps left.
    budget: &'w mut Budget,
    /// How deep the value being decoded is.
    depth: Depth,
    /// Payload octets postcard has taken.
    at: Rc<Cell<usize>>,
    /// The first limit the decode broke. A type may swallow the error that
    /// reports it, so [`decode`] looks here as well.
    fault: Option<ParseFault>,
}

impl Watch<'_> {
    /// Payload octets taken so far.
    fn offset(&self) -> u64 {
        u64::try_from(self.at.get()).unwrap_or(u64::MAX)
    }

    /// Records `fault`, keeping the first one, and returns the error that
    /// stops the decode.
    fn fail<E: de::Error>(&mut self, fault: ParseFault) -> E {
        self.fault.get_or_insert(fault);
        E::custom("the wire guard stopped the decode")
    }

    /// Spends one step on a value that starts here and goes one level
    /// deeper for it, returning the depth to go back to once it is done.
    fn enter<E: de::Error>(&mut self) -> Result<Depth, E> {
        let offset = self.offset();
        let outer = self.depth;
        let limits = self.limits;
        match self
            .budget
            .charge(1, offset)
            .and_then(|()| outer.descend(limits, offset))
        {
            Ok(inner) => {
                self.depth = inner;
                Ok(outer)
            }
            Err(fault) => Err(self.fail(fault)),
        }
    }

    /// Checks a decoded string of `len` octets, which ended here, against
    /// [`LimitKind::LongText`].
    fn text<E: de::Error>(&mut self, len: usize) -> Result<(), E> {
        let value = u64::try_from(len).unwrap_or(u64::MAX);
        let max = self.limits.get(LimitKind::LongText);
        if value > max {
            let offset = self.offset().saturating_sub(value);
            return Err(self.fail(ParseFault::LimitExceeded {
                limit: LimitKind::LongText,
                value,
                max,
                offset,
            }));
        }
        Ok(())
    }

    /// Counts one more entry of a sequence or map that started at `start`,
    /// after `count` entries, against [`LimitKind::Children`].
    fn entry<E: de::Error>(&mut self, count: u64, start: u64) -> Result<u64, E> {
        let value = count.saturating_add(1);
        let max = self.limits.get(LimitKind::Children);
        if value > max {
            return Err(self.fail(ParseFault::LimitExceeded {
                limit: LimitKind::Children,
                value,
                max,
                offset: start,
            }));
        }
        Ok(value)
    }
}

/// A deserializer that charges every value to the [`Watch`] before handing
/// it to postcard.
struct Guard<'a, 'w, D> {
    /// Postcard's deserializer, or one it lent for a nested value.
    inner: D,
    /// The decode's state.
    watch: &'a mut Watch<'w>,
}

impl<'de, 'w, D: Deserializer<'de>> Guard<'_, 'w, D> {
    /// Enters one value, lets `call` hand it to the inner deserializer with
    /// a guarded visitor, and leaves it again.
    fn value<V: Visitor<'de>>(
        self,
        visitor: V,
        call: impl for<'x> FnOnce(D, Visit<'x, 'w, V>) -> Result<V::Value, D::Error>,
    ) -> Result<V::Value, D::Error> {
        let Self { inner, watch } = self;
        let outer = watch.enter()?;
        let value = call(
            inner,
            Visit {
                inner: visitor,
                watch: &mut *watch,
            },
        );
        watch.depth = outer;
        value
    }
}

impl<'de, D: Deserializer<'de>> Deserializer<'de> for Guard<'_, '_, D> {
    type Error = D::Error;

    fn is_human_readable(&self) -> bool {
        false
    }

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_any(visit))
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_bool(visit))
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_i8(visit))
    }

    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_i16(visit))
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_i32(visit))
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_i64(visit))
    }

    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_i128(visit))
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_u8(visit))
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_u16(visit))
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_u32(visit))
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_u64(visit))
    }

    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_u128(visit))
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_f32(visit))
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_f64(visit))
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_char(visit))
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_str(visit))
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_string(visit))
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_bytes(visit))
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_byte_buf(visit))
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_option(visit))
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_unit(visit))
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| {
            inner.deserialize_unit_struct(name, visit)
        })
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| {
            inner.deserialize_newtype_struct(name, visit)
        })
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_seq(visit))
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_tuple(len, visit))
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| {
            inner.deserialize_tuple_struct(name, len, visit)
        })
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_map(visit))
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| {
            inner.deserialize_struct(name, fields, visit)
        })
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| {
            inner.deserialize_enum(name, variants, visit)
        })
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_identifier(visit))
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.value(visitor, |inner, visit| inner.deserialize_ignored_any(visit))
    }
}

/// A visitor that checks strings and guards the nested values a visited
/// value hands on, then passes everything to the type's own visitor.
///
/// It forwards exactly the calls postcard makes.
struct Visit<'a, 'w, V> {
    /// The type's own visitor.
    inner: V,
    /// The decode's state.
    watch: &'a mut Watch<'w>,
}

impl<'de, V: Visitor<'de>> Visitor<'de> for Visit<'_, '_, V> {
    type Value = V::Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.expecting(formatter)
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<V::Value, E> {
        self.inner.visit_bool(v)
    }

    fn visit_i8<E: de::Error>(self, v: i8) -> Result<V::Value, E> {
        self.inner.visit_i8(v)
    }

    fn visit_i16<E: de::Error>(self, v: i16) -> Result<V::Value, E> {
        self.inner.visit_i16(v)
    }

    fn visit_i32<E: de::Error>(self, v: i32) -> Result<V::Value, E> {
        self.inner.visit_i32(v)
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<V::Value, E> {
        self.inner.visit_i64(v)
    }

    fn visit_i128<E: de::Error>(self, v: i128) -> Result<V::Value, E> {
        self.inner.visit_i128(v)
    }

    fn visit_u8<E: de::Error>(self, v: u8) -> Result<V::Value, E> {
        self.inner.visit_u8(v)
    }

    fn visit_u16<E: de::Error>(self, v: u16) -> Result<V::Value, E> {
        self.inner.visit_u16(v)
    }

    fn visit_u32<E: de::Error>(self, v: u32) -> Result<V::Value, E> {
        self.inner.visit_u32(v)
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<V::Value, E> {
        self.inner.visit_u64(v)
    }

    fn visit_u128<E: de::Error>(self, v: u128) -> Result<V::Value, E> {
        self.inner.visit_u128(v)
    }

    fn visit_f32<E: de::Error>(self, v: f32) -> Result<V::Value, E> {
        self.inner.visit_f32(v)
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<V::Value, E> {
        self.inner.visit_f64(v)
    }

    fn visit_char<E: de::Error>(self, v: char) -> Result<V::Value, E> {
        self.inner.visit_char(v)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<V::Value, E> {
        self.watch.text(v.len())?;
        self.inner.visit_str(v)
    }

    fn visit_borrowed_str<E: de::Error>(self, v: &'de str) -> Result<V::Value, E> {
        self.watch.text(v.len())?;
        self.inner.visit_borrowed_str(v)
    }

    fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<V::Value, E> {
        self.inner.visit_bytes(v)
    }

    fn visit_borrowed_bytes<E: de::Error>(self, v: &'de [u8]) -> Result<V::Value, E> {
        self.inner.visit_borrowed_bytes(v)
    }

    fn visit_none<E: de::Error>(self) -> Result<V::Value, E> {
        self.inner.visit_none()
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<V::Value, D::Error> {
        self.inner.visit_some(Guard {
            inner: deserializer,
            watch: self.watch,
        })
    }

    fn visit_unit<E: de::Error>(self) -> Result<V::Value, E> {
        self.inner.visit_unit()
    }

    fn visit_newtype_struct<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<V::Value, D::Error> {
        self.inner.visit_newtype_struct(Guard {
            inner: deserializer,
            watch: self.watch,
        })
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<V::Value, A::Error> {
        let start = self.watch.offset();
        self.inner.visit_seq(Entries {
            inner: seq,
            watch: self.watch,
            count: 0,
            start,
        })
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<V::Value, A::Error> {
        let start = self.watch.offset();
        self.inner.visit_map(Entries {
            inner: map,
            watch: self.watch,
            count: 0,
            start,
        })
    }

    fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<V::Value, A::Error> {
        self.inner.visit_enum(Variant {
            inner: data,
            watch: self.watch,
        })
    }
}

/// The entries of a sequence or map, counted against
/// [`LimitKind::Children`], each guarded.
struct Entries<'a, 'w, A> {
    /// Postcard's access to the entries.
    inner: A,
    /// The decode's state.
    watch: &'a mut Watch<'w>,
    /// Entries decoded so far.
    count: u64,
    /// Where the sequence or map's entries start in the payload.
    start: u64,
}

impl<'de, A: SeqAccess<'de>> SeqAccess<'de> for Entries<'_, '_, A> {
    type Error = A::Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, A::Error> {
        let next = self.inner.next_element_seed(Seed {
            inner: seed,
            watch: &mut *self.watch,
        })?;
        if next.is_some() {
            self.count = self.watch.entry(self.count, self.start)?;
        }
        Ok(next)
    }
}

impl<'de, A: MapAccess<'de>> MapAccess<'de> for Entries<'_, '_, A> {
    type Error = A::Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, A::Error> {
        let next = self.inner.next_key_seed(Seed {
            inner: seed,
            watch: &mut *self.watch,
        })?;
        if next.is_some() {
            self.count = self.watch.entry(self.count, self.start)?;
        }
        Ok(next)
    }

    fn next_value_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<T::Value, A::Error> {
        self.inner.next_value_seed(Seed {
            inner: seed,
            watch: &mut *self.watch,
        })
    }
}

/// An enum's variant, whose content is guarded like any other value.
struct Variant<'a, 'w, A> {
    /// Postcard's access to the variant.
    inner: A,
    /// The decode's state.
    watch: &'a mut Watch<'w>,
}

impl<'a, 'w, 'de, A: EnumAccess<'de>> EnumAccess<'de> for Variant<'a, 'w, A> {
    type Error = A::Error;
    type Variant = Variant<'a, 'w, A::Variant>;

    fn variant_seed<T: DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<(T::Value, Self::Variant), A::Error> {
        let (value, variant) = self.inner.variant_seed(seed)?;
        Ok((
            value,
            Variant {
                inner: variant,
                watch: self.watch,
            },
        ))
    }
}

impl<'de, A: VariantAccess<'de>> VariantAccess<'de> for Variant<'_, '_, A> {
    type Error = A::Error;

    /// A unit variant's content is a value of no octets, charged like a
    /// unit.
    fn unit_variant(self) -> Result<(), A::Error> {
        let Self { inner, watch } = self;
        let outer = watch.enter()?;
        let done = inner.unit_variant();
        watch.depth = outer;
        done
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, A::Error> {
        self.inner.newtype_variant_seed(Seed {
            inner: seed,
            watch: self.watch,
        })
    }

    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, A::Error> {
        self.inner.tuple_variant(
            len,
            Visit {
                inner: visitor,
                watch: self.watch,
            },
        )
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, A::Error> {
        self.inner.struct_variant(
            fields,
            Visit {
                inner: visitor,
                watch: self.watch,
            },
        )
    }
}

/// A seed whose value is decoded through a [`Guard`].
struct Seed<'a, 'w, T> {
    /// The type's own seed.
    inner: T,
    /// The decode's state.
    watch: &'a mut Watch<'w>,
}

impl<'de, T: DeserializeSeed<'de>> DeserializeSeed<'de> for Seed<'_, '_, T> {
    type Value = T::Value;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<T::Value, D::Error> {
        self.inner.deserialize(Guard {
            inner: deserializer,
            watch: self.watch,
        })
    }
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use std::collections::BTreeMap;
    use std::net::Ipv4Addr;

    use proptest::collection::vec;
    use proptest::prelude::*;
    use serde::de::{Expected, IgnoredAny};

    use super::*;

    /// The stack size SEC-MED-001 names, in octets.
    const SMALL_STACK: usize = 262_144;

    /// Runs `work` on a thread with a 256 KiB stack, so a decode that
    /// recursed without bound would fail here instead of passing on the
    /// test runner's larger stack.
    fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(SMALL_STACK)
            .spawn(work)
            .expect("the test thread starts")
            .join()
            .expect("the code under test returned instead of panicking")
    }

    /// `value` in postcard's encoding, written by postcard itself rather
    /// than by the code under test.
    fn encode<T: Serialize + ?Sized>(value: &T) -> Vec<u8> {
        postcard::to_extend(value, Vec::new()).expect("the test value encodes")
    }

    /// `len` copies of `octet`.
    fn filled(octet: u8, len: usize) -> Vec<u8> {
        (0..len).map(|_| octet).collect()
    }

    /// A budget large enough for any test payload.
    fn plenty() -> Budget {
        Budget::for_input(0, 0, 1 << 40)
    }

    /// Decodes `payload` under the default limits and a budget of `steps`,
    /// returning the result and the steps spent.
    fn decode_with<'de, T: Deserialize<'de>>(
        payload: &'de [u8],
        limits: &Limits,
        steps: u64,
    ) -> (Result<T, WireError>, u64) {
        let mut budget = Budget::for_input(0, 0, steps);
        let decoded = decode(payload, limits, &mut budget);
        (decoded, steps - budget.remaining())
    }

    /// Default limits with `kind` lowered to `value`.
    fn lowered(kind: LimitKind, value: u64) -> Limits {
        Limits::DEFAULT
            .with_override(kind, value)
            .expect("lowering a limit is allowed")
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Track<'a> {
        id: u32,
        title: &'a str,
        disc: Option<u8>,
    }

    /// The `Track` the literal encodings below hold.
    const TRACK: Track<'static> = Track {
        id: 300,
        title: "Hi",
        disc: Some(2),
    };

    /// `TRACK` by hand: 300 as a varint, a two-octet string, `Some(2)`.
    const TRACK_OCTETS: [u8; 7] = [0xAC, 0x02, 0x02, b'H', b'i', 0x01, 0x02];

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Marker;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Wrapper(u16);

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Pair(u8, u8);

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum Choice {
        Unit,
        Newtype(u8),
        Tuple(u8, u8),
        Struct { x: u8 },
    }

    /// Octets written with `serialize_bytes` and read through
    /// `deserialize_byte_buf`.
    #[derive(Debug, PartialEq)]
    struct Buf(Vec<u8>);

    impl Serialize for Buf {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_bytes(&self.0)
        }
    }

    impl<'de> Deserialize<'de> for Buf {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            deserializer.deserialize_byte_buf(BufVisitor)
        }
    }

    struct BufVisitor;

    impl Visitor<'_> for BufVisitor {
        type Value = Buf;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("octets")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Buf, E> {
            Ok(Buf(v.to_vec()))
        }
    }

    /// One value of every shape serde's data model has.
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Everything<'a> {
        yes: bool,
        small: i8,
        short: i16,
        int: i32,
        long: i64,
        huge: i128,
        octet: u8,
        ushort: u16,
        uint: u32,
        ulong: u64,
        uhuge: u128,
        single: f32,
        double: f64,
        letter: char,
        borrowed: &'a str,
        owned: String,
        octets: &'a [u8],
        buf: Buf,
        none: Option<u8>,
        some: Option<u8>,
        unit: (),
        marker: Marker,
        wrapper: Wrapper,
        list: Vec<u16>,
        pair: (u8, u8),
        tuple_struct: Pair,
        map: BTreeMap<u8, u8>,
        choices: [Choice; 4],
        addr: Ipv4Addr,
    }

    fn everything() -> Everything<'static> {
        Everything {
            yes: true,
            small: -5,
            short: -300,
            int: -70_000,
            long: -5_000_000_000,
            huge: -(1 << 100),
            octet: 200,
            ushort: 60_000,
            uint: 4_000_000_000,
            ulong: 1 << 60,
            uhuge: 1 << 120,
            single: 1.5,
            double: -2.25,
            letter: 'é',
            borrowed: "borrowed",
            owned: "owned".to_owned(),
            octets: &[1, 2, 3],
            buf: Buf(vec![4, 5]),
            none: None,
            some: Some(9),
            unit: (),
            marker: Marker,
            wrapper: Wrapper(513),
            list: vec![300, 7],
            pair: (1, 2),
            tuple_struct: Pair(3, 4),
            map: BTreeMap::from([(1, 2), (3, 4)]),
            choices: [
                Choice::Unit,
                Choice::Newtype(6),
                Choice::Tuple(7, 8),
                Choice::Struct { x: 9 },
            ],
            addr: Ipv4Addr::new(192, 0, 2, 1),
        }
    }

    /// A tree whose depth the input chooses: each `Node` is one more enum
    /// level, and the `Leaf` at the bottom is an enum level with a unit
    /// variant's content below it.
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum Tree {
        Leaf,
        Node(Box<Self>),
    }

    /// `Node` `nodes` times around a `Leaf`, built without recursion.
    fn tree(nodes: usize) -> Tree {
        (0..nodes).fold(Tree::Leaf, |inner, _| Tree::Node(Box::new(inner)))
    }

    /// The encoding of `tree(nodes)` by hand: one octet 1 per `Node`, then
    /// the octet 0 of the `Leaf`.
    fn tree_octets(nodes: usize) -> Vec<u8> {
        let mut octets = filled(1, nodes);
        octets.push(0);
        octets
    }

    // Frames.

    #[test]
    fn writes_a_frame_as_length_version_kind_and_payload() {
        assert_eq!(
            write_frame(ProtocolVersion(1), FrameKind::Response, &TRACK),
            Ok(vec![
                10, 0, 0, 0, 1, 0, 2, 0xAC, 0x02, 0x02, b'H', b'i', 0x01, 0x02
            ])
        );
        assert_eq!(
            write_frame(ProtocolVersion(0x0102), FrameKind::End, &()),
            Ok(vec![3, 0, 0, 0, 0x02, 0x01, 4])
        );
    }

    #[test]
    fn reads_the_frame_at_the_start_of_its_input() {
        let input = [
            10, 0, 0, 0, 0x02, 0x01, 1, 0xAC, 0x02, 0x02, b'H', b'i', 0x01, 0x02, 0xEE,
        ];
        let mut budget = Budget::for_input(0, 0, 5);
        assert_eq!(
            read_frame(&input, MAX_FRAME, &mut budget),
            Ok(Frame {
                version: ProtocolVersion(0x0102),
                kind: FrameKind::Request,
                payload: &TRACK_OCTETS,
                len: 14,
            })
        );
        assert_eq!(budget.remaining(), 4);
        assert_eq!(
            read_frame(&[3, 0, 0, 0, 1, 0, 5], MAX_FRAME, &mut budget),
            Ok(Frame {
                version: ProtocolVersion(1),
                kind: FrameKind::Refusal,
                payload: &[],
                len: 7,
            })
        );
    }

    #[test]
    fn every_kind_has_one_octet_and_no_other_octet_is_a_kind() {
        let kinds = [
            (1, FrameKind::Request),
            (2, FrameKind::Response),
            (3, FrameKind::Part),
            (4, FrameKind::End),
            (5, FrameKind::Refusal),
        ];
        for octet in 0..=u8::MAX {
            let expected = kinds
                .iter()
                .find(|(written, _)| *written == octet)
                .map(|(_, kind)| *kind);
            assert_eq!(FrameKind::from_octet(octet), expected);
        }
        for (octet, kind) in kinds {
            assert_eq!(kind.octet(), octet);
        }
    }

    #[test]
    fn refuses_a_kind_it_does_not_know() {
        for kind in [0, 6, 0xFF] {
            assert_eq!(
                read_frame(&[3, 0, 0, 0, 1, 0, kind], MAX_FRAME, &mut plenty()),
                Err(WireError::UnknownKind { kind })
            );
        }
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn refuses_a_length_smaller_than_the_version_and_kind() {
        for (len, input) in [
            (0, vec![0, 0, 0, 0]),
            (1, vec![1, 0, 0, 0, 1]),
            (2, vec![2, 0, 0, 0, 1, 0]),
        ] {
            assert_eq!(
                read_frame(&input, MAX_FRAME, &mut plenty()),
                Err(WireError::TooShort { len })
            );
        }
    }

    #[test]
    fn honours_a_cap_lower_than_the_frame_cap() {
        let mut at_cap = vec![10, 0, 0, 0, 1, 0, 2];
        at_cap.extend([0; 7]);
        assert_eq!(
            read_frame(&at_cap, 10, &mut plenty()),
            Ok(Frame {
                version: ProtocolVersion(1),
                kind: FrameKind::Response,
                payload: &[0; 7],
                len: 14,
            })
        );
        assert_eq!(
            read_frame(&at_cap, 9, &mut plenty()),
            Err(WireError::TooLarge { len: 10, max: 9 })
        );
    }

    /// Verifies: SEC-MED-023
    #[test]
    fn accepts_a_32_mib_frame_and_refuses_one_octet_more_whatever_cap_is_asked_for() {
        let mut input = MAX_FRAME.to_le_bytes().to_vec();
        input.extend([1, 0, 3]);
        input.extend(filled(0xA5, 33_554_429));
        let frame = read_frame(&input, u32::MAX, &mut plenty());
        assert_eq!(
            frame.map(|frame| (frame.version, frame.kind, frame.payload.len(), frame.len)),
            Ok((ProtocolVersion(1), FrameKind::Part, 33_554_429, 33_554_436))
        );
        let over = (MAX_FRAME + 1).to_le_bytes();
        assert_eq!(
            read_frame(&over, u32::MAX, &mut plenty()),
            Err(WireError::TooLarge {
                len: 33_554_433,
                max: MAX_FRAME,
            })
        );
    }

    /// Verifies: SEC-MED-023, SEC-TM-032
    #[test]
    fn refuses_a_four_gigabyte_length_before_any_payload_arrives() {
        assert_eq!(
            read_frame(&[0xFF, 0xFF, 0xFF, 0xFF], MAX_FRAME, &mut plenty()),
            Err(WireError::TooLarge {
                len: 4_294_967_295,
                max: MAX_FRAME,
            })
        );
    }

    #[test]
    fn says_how_much_more_a_frame_cut_at_any_octet_needs() {
        let whole = [
            10, 0, 0, 0, 1, 0, 2, 0xAC, 0x02, 0x02, b'H', b'i', 0x01, 0x02,
        ];
        for cut in 0..whole.len() {
            let expected = if cut < 4 {
                ParseFault::Truncated {
                    offset: 0,
                    needed: 4,
                    available: cut as u64,
                }
            } else {
                ParseFault::Truncated {
                    offset: 4,
                    needed: 10,
                    available: cut as u64 - 4,
                }
            };
            assert_eq!(
                read_frame(&whole[..cut], MAX_FRAME, &mut plenty()),
                Err(WireError::Fault(expected))
            );
        }
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn spends_one_step_per_frame_and_stops_when_none_are_left() {
        let frame = [3, 0, 0, 0, 1, 0, 4];
        let end = Ok(Frame {
            version: ProtocolVersion(1),
            kind: FrameKind::End,
            payload: &[],
            len: 7,
        });
        let mut budget = Budget::for_input(0, 0, 2);
        assert_eq!(read_frame(&frame, MAX_FRAME, &mut budget), end);
        assert_eq!(budget.remaining(), 1);
        assert_eq!(read_frame(&frame, MAX_FRAME, &mut budget), end);
        assert_eq!(budget.remaining(), 0);
        assert_eq!(
            read_frame(&frame, MAX_FRAME, &mut budget),
            Err(WireError::Fault(ParseFault::BudgetExceeded { offset: 0 }))
        );
    }

    /// Octets written with `serialize_bytes` from a borrowed slice.
    struct Octets<'a>(&'a [u8]);

    impl Serialize for Octets<'_> {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_bytes(self.0)
        }
    }

    #[test]
    fn writes_a_32_mib_frame_and_refuses_one_octet_more() {
        // 33,554,425 octets take a four-octet varint: 4 + 33,554,425 + 3
        // is exactly the cap.
        let octets = filled(0x5A, 33_554_425);
        let frame = write_frame(ProtocolVersion(1), FrameKind::Part, &Octets(&octets));
        assert_eq!(
            frame.map(|frame| (frame.len(), frame[..11].to_vec())),
            Ok((
                33_554_436,
                vec![0x00, 0x00, 0x00, 0x02, 1, 0, 3, 0xF9, 0xFF, 0xFF, 0x0F]
            ))
        );
        let over = filled(0x5A, 33_554_426);
        assert_eq!(
            write_frame(ProtocolVersion(1), FrameKind::Part, &Octets(&over)),
            Err(WireError::TooLarge {
                len: 33_554_433,
                max: MAX_FRAME,
            })
        );
    }

    /// A sequence that does not say how long it is, which postcard cannot
    /// write.
    struct Unsized;

    impl Serialize for Unsized {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer
                .serialize_seq(None)
                .and_then(serde::ser::SerializeSeq::end)
        }
    }

    #[test]
    fn reports_a_value_postcard_cannot_write() {
        assert_eq!(
            write_frame(ProtocolVersion(1), FrameKind::Request, &Unsized),
            Err(WireError::Unencodable {
                reason: postcard::Error::SerializeSeqLengthUnknown,
            })
        );
    }

    /// Reads frames from `input` one after another until it is used up or
    /// a frame is refused, failing if that takes more reads than a stream
    /// of seven-octet frames could need.
    fn read_all(input: &[u8]) -> Vec<Result<Frame<'_>, WireError>> {
        let ceiling = input.len() / 7 + 1;
        let mut frames = Vec::new();
        let mut rest = input;
        while !rest.is_empty() {
            assert!(frames.len() < ceiling);
            let frame = read_frame(rest, MAX_FRAME, &mut plenty());
            // Nothing is read after a frame that was refused.
            let taken = frame.as_ref().map_or(rest.len(), |frame| frame.len);
            frames.push(frame);
            rest = &rest[taken..];
        }
        frames
    }

    #[test]
    fn reads_a_stream_of_frames_one_after_another() {
        let mut stream = vec![3, 0, 0, 0, 1, 0, 3];
        stream.extend([4, 0, 0, 0, 1, 0, 3, 0xAB]);
        stream.extend([3, 0, 0, 0, 1, 0, 4]);
        assert_eq!(
            read_all(&stream),
            [
                Ok(Frame {
                    version: ProtocolVersion(1),
                    kind: FrameKind::Part,
                    payload: &[],
                    len: 7,
                }),
                Ok(Frame {
                    version: ProtocolVersion(1),
                    kind: FrameKind::Part,
                    payload: &[0xAB],
                    len: 8,
                }),
                Ok(Frame {
                    version: ProtocolVersion(1),
                    kind: FrameKind::End,
                    payload: &[],
                    len: 7,
                }),
            ]
        );
    }

    // Payloads.

    #[test]
    fn decodes_a_payload_as_the_type_asked_for() {
        let (decoded, steps) = decode_with::<Track<'_>>(&TRACK_OCTETS, &Limits::DEFAULT, 100);
        assert_eq!(decoded, Ok(TRACK));
        // The struct, its three fields, and the content of `Some`.
        assert_eq!(steps, 5);
    }

    #[test]
    fn decodes_every_shape_of_value_and_charges_one_step_for_each() {
        let octets = encode(&everything());
        let (decoded, steps) = decode_with::<Everything<'_>>(&octets, &Limits::DEFAULT, 1_000);
        assert_eq!(decoded, Ok(everything()));
        // The struct (1); 18 scalar, string and octet fields (18); `None`
        // (1) and `Some` with its content (2); the unit and unit struct
        // (2); the newtype and its content (2); the list and its two
        // entries (3); the tuple and the tuple struct, two entries each
        // (6); the map and two keys and two values (5); the array of
        // choices (1) and its four choices: a unit variant and its empty
        // content (2), a newtype variant and its content (2), a tuple
        // variant and its two entries (3), a struct variant and its field
        // (2); the address as four octets (5).
        assert_eq!(steps, 55);
    }

    #[test]
    fn says_what_the_type_expected() {
        let mut budget = plenty();
        let at = Rc::new(Cell::new(0));
        let mut watch = Watch {
            limits: &Limits::DEFAULT,
            budget: &mut budget,
            depth: Depth::CONTAINER_ROOT,
            at,
            fault: None,
        };
        let visit = Visit {
            inner: BufVisitor,
            watch: &mut watch,
        };
        assert_eq!(format!("{}", &visit as &dyn Expected), "octets");
    }

    /// Verifies: SEC-HIS-035
    #[test]
    fn refuses_a_payload_with_fields_the_type_does_not_have() {
        #[derive(Serialize)]
        struct Newer {
            id: u32,
            title: &'static str,
            disc: Option<u8>,
            added: u8,
        }
        let newer = encode(&Newer {
            id: 300,
            title: "Hi",
            disc: Some(2),
            added: 7,
        });
        assert_eq!(
            decode::<Track<'_>>(&newer, &Limits::DEFAULT, &mut plenty()),
            Err(WireError::Trailing { offset: 7, len: 1 })
        );
        assert_eq!(
            decode::<u8>(&[1, 2, 3], &Limits::DEFAULT, &mut plenty()),
            Err(WireError::Trailing { offset: 1, len: 2 })
        );
    }

    #[test]
    fn reports_where_postcard_found_the_payload_malformed() {
        assert_eq!(
            decode::<bool>(&[2], &Limits::DEFAULT, &mut plenty()),
            Err(WireError::Malformed {
                offset: 1,
                reason: postcard::Error::DeserializeBadBool,
            })
        );
        assert_eq!(
            decode::<Track<'_>>(&TRACK_OCTETS[..4], &Limits::DEFAULT, &mut plenty()),
            Err(WireError::Malformed {
                offset: 3,
                reason: postcard::Error::DeserializeUnexpectedEnd,
            })
        );
        assert_eq!(
            decode::<u8>(&[], &Limits::DEFAULT, &mut plenty()),
            Err(WireError::Malformed {
                offset: 0,
                reason: postcard::Error::DeserializeUnexpectedEnd,
            })
        );
    }

    /// A declared length of `u32::MAX` would ask for four gigabytes; the
    /// length is checked against what is left of the payload, and the
    /// decode fails where the payload runs out.
    ///
    /// Verifies: SEC-TM-032
    #[test]
    fn refuses_a_declared_length_longer_than_the_payload() {
        let mut octets = vec![0xFF, 0xFF, 0xFF, 0xFF, 0x0F];
        octets.extend(b"short");
        let ends_at = |offset| WireError::Malformed {
            offset,
            reason: postcard::Error::DeserializeUnexpectedEnd,
        };
        assert_eq!(
            decode::<&str>(&octets, &Limits::DEFAULT, &mut plenty()),
            Err(ends_at(5))
        );
        assert_eq!(
            decode::<String>(&octets, &Limits::DEFAULT, &mut plenty()),
            Err(ends_at(5))
        );
        assert_eq!(
            decode::<&[u8]>(&octets, &Limits::DEFAULT, &mut plenty()),
            Err(ends_at(5))
        );
        // Five one-octet entries, then the payload ends inside the sixth.
        assert_eq!(
            decode::<Vec<u32>>(&octets, &Limits::DEFAULT, &mut plenty()),
            Err(ends_at(10))
        );
    }

    /// The octets of a list or a map, and what the type was told about its
    /// length before it read the first entry.
    #[derive(Debug, PartialEq)]
    struct Hinted {
        hint: Option<usize>,
        octets: Vec<u8>,
    }

    /// A [`Hinted`] read as a sequence.
    #[derive(Debug, PartialEq)]
    struct HintedList(Hinted);

    impl<'de> Deserialize<'de> for HintedList {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            deserializer.deserialize_seq(HintedVisitor).map(Self)
        }
    }

    /// A [`Hinted`] read as a map.
    #[derive(Debug, PartialEq)]
    struct HintedMap(Hinted);

    impl<'de> Deserialize<'de> for HintedMap {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            deserializer.deserialize_map(HintedVisitor).map(Self)
        }
    }

    struct HintedVisitor;

    impl<'de> Visitor<'de> for HintedVisitor {
        type Value = Hinted;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("entries")
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Hinted, A::Error> {
            let hint = seq.size_hint();
            let mut octets = Vec::new();
            while let Ok(Some(octet)) = seq.next_element::<u8>() {
                octets.push(octet);
            }
            Ok(Hinted { hint, octets })
        }

        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Hinted, A::Error> {
            let hint = map.size_hint();
            let mut octets = Vec::new();
            while let Ok(Some((key, value))) = map.next_entry::<u8, u8>() {
                octets.extend([key, value]);
            }
            Ok(Hinted { hint, octets })
        }
    }

    /// A type cannot size an allocation from a count the payload declares,
    /// because the guard never passes the count on.
    #[test]
    fn keeps_a_declared_count_from_the_type_being_decoded() {
        assert_eq!(format!("{}", &HintedVisitor as &dyn Expected), "entries");
        assert_eq!(
            decode::<HintedList>(&[3, 7, 8, 9], &Limits::DEFAULT, &mut plenty()),
            Ok(HintedList(Hinted {
                hint: None,
                octets: vec![7, 8, 9],
            }))
        );
        assert_eq!(
            decode::<HintedMap>(&[2, 1, 2, 3, 4], &Limits::DEFAULT, &mut plenty()),
            Ok(HintedMap(Hinted {
                hint: None,
                octets: vec![1, 2, 3, 4],
            }))
        );
    }

    /// Three types, each asking for one self-describing call that postcard
    /// does not make.
    #[derive(Debug, PartialEq)]
    struct AsksForAny;

    impl<'de> Deserialize<'de> for AsksForAny {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            deserializer.deserialize_any(IgnoredAny).map(|_| Self)
        }
    }

    #[derive(Debug, PartialEq)]
    struct AsksForIdentifier;

    impl<'de> Deserialize<'de> for AsksForIdentifier {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            deserializer
                .deserialize_identifier(IgnoredAny)
                .map(|_| Self)
        }
    }

    #[derive(Debug, PartialEq)]
    struct AsksToIgnore;

    impl<'de> Deserialize<'de> for AsksToIgnore {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            deserializer
                .deserialize_ignored_any(IgnoredAny)
                .map(|_| Self)
        }
    }

    #[test]
    fn passes_self_describing_calls_to_postcard_which_refuses_them() {
        let refused = WireError::Malformed {
            offset: 0,
            reason: postcard::Error::WontImplement,
        };
        assert_eq!(
            decode::<AsksForAny>(&[1], &Limits::DEFAULT, &mut plenty()),
            Err(refused.clone())
        );
        assert_eq!(
            decode::<AsksForIdentifier>(&[1], &Limits::DEFAULT, &mut plenty()),
            Err(refused.clone())
        );
        assert_eq!(
            decode::<AsksToIgnore>(&[1], &Limits::DEFAULT, &mut plenty()),
            Err(refused)
        );
    }

    /// Verifies: SEC-MED-023, SEC-CLI-021
    #[test]
    fn decodes_a_32_mib_payload_and_refuses_one_octet_more() {
        // A four-octet varint and 33,554,428 octets fill the cap exactly.
        let mut payload = vec![0xFC, 0xFF, 0xFF, 0x0F];
        payload.extend(filled(0x33, 33_554_428));
        let decoded = decode::<&[u8]>(&payload, &Limits::DEFAULT, &mut plenty());
        assert_eq!(decoded.map(<[u8]>::len), Ok(33_554_428));
        payload.push(0x33);
        let too_large = WireError::TooLarge {
            len: 33_554_433,
            max: MAX_FRAME,
        };
        assert_eq!(
            decode::<&[u8]>(&payload, &Limits::DEFAULT, &mut plenty()),
            Err(too_large.clone())
        );
        // Whatever the type asked for, and before any of it is read.
        let mut budget = Budget::for_input(0, 0, 9);
        assert_eq!(
            decode::<Track<'_>>(&payload, &Limits::DEFAULT, &mut budget),
            Err(too_large)
        );
        assert_eq!(budget.remaining(), 9);
    }

    /// Verifies: SEC-MED-001, SEC-MED-005, SEC-CLI-021
    #[test]
    fn nests_32_levels_and_refuses_the_33rd_on_a_small_stack() {
        let (at_max, too_deep, endless) = on_small_stack(|| {
            let decode_tree = |octets: &[u8]| decode_with::<Tree>(octets, &Limits::DEFAULT, 1_000);
            (
                decode_tree(&tree_octets(30)),
                decode_tree(&tree_octets(31)),
                decode_tree(&[1; 10_000]),
            )
        });
        // Thirty-one enums and the leaf's empty content: 32 levels.
        assert_eq!(at_max, (Ok(tree(30)), 32));
        let refused = WireError::Fault(ParseFault::TooDeep {
            limit: LimitKind::ContainerDepth,
            depth: 33,
            max: 32,
            offset: 32,
        });
        assert_eq!(too_deep, (Err(refused.clone()), 33));
        assert_eq!(endless, (Err(refused), 33));
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn a_value_deep_inside_its_sibling_does_not_count_against_the_next() {
        let mut octets = tree_octets(29);
        octets.extend(tree_octets(29));
        assert_eq!(
            decode::<(Tree, Tree)>(&octets, &Limits::DEFAULT, &mut plenty()),
            Ok((tree(29), tree(29)))
        );
    }

    /// Verifies: SEC-MED-006, SEC-CLI-021
    #[test]
    fn counts_the_entries_of_a_sequence_up_to_the_children_limit() {
        let mut at_max = vec![0x80, 0x80, 0x04];
        at_max.extend(filled(7, 65_536));
        let decoded = decode::<Vec<u8>>(&at_max, &Limits::DEFAULT, &mut plenty());
        assert_eq!(decoded.map(|entries| entries.len()), Ok(65_536));
        let mut over = vec![0x81, 0x80, 0x04];
        over.extend(filled(7, 65_537));
        assert_eq!(
            decode::<Vec<u8>>(&over, &Limits::DEFAULT, &mut plenty()),
            Err(WireError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::Children,
                value: 65_537,
                max: 65_536,
                offset: 3,
            }))
        );
    }

    /// Verifies: SEC-MED-006, SEC-CLI-021
    #[test]
    fn counts_the_entries_of_a_map_and_of_a_sequence_against_lowered_limits() {
        let two = lowered(LimitKind::Children, 2);
        let map = encode(&BTreeMap::from([(1_u8, 2_u8), (3, 4)]));
        assert_eq!(
            decode::<BTreeMap<u8, u8>>(&map, &two, &mut plenty()),
            Ok(BTreeMap::from([(1, 2), (3, 4)]))
        );
        let map = encode(&BTreeMap::from([(1_u8, 2_u8), (3, 4), (5, 6)]));
        let refused = WireError::Fault(ParseFault::LimitExceeded {
            limit: LimitKind::Children,
            value: 3,
            max: 2,
            offset: 1,
        });
        assert_eq!(
            decode::<BTreeMap<u8, u8>>(&map, &two, &mut plenty()),
            Err(refused.clone())
        );
        assert_eq!(
            decode::<Vec<u8>>(&[3, 1, 2, 3], &two, &mut plenty()),
            Err(refused)
        );
    }

    /// Verifies: SEC-MED-006, SEC-CLI-021
    #[test]
    fn holds_every_decoded_string_to_the_long_text_limit() {
        let mut at_max = vec![0x80, 0x80, 0x04];
        at_max.extend(filled(b'a', 65_536));
        let mut over = vec![0x81, 0x80, 0x04];
        over.extend(filled(b'a', 65_537));
        let refused = WireError::Fault(ParseFault::LimitExceeded {
            limit: LimitKind::LongText,
            value: 65_537,
            max: 65_536,
            offset: 3,
        });
        let borrowed = decode::<&str>(&at_max, &Limits::DEFAULT, &mut plenty());
        assert_eq!(borrowed.map(str::len), Ok(65_536));
        assert_eq!(
            decode::<&str>(&over, &Limits::DEFAULT, &mut plenty()),
            Err(refused.clone())
        );
        let owned = decode::<String>(&at_max, &Limits::DEFAULT, &mut plenty());
        assert_eq!(owned.map(|text| text.len()), Ok(65_536));
        assert_eq!(
            decode::<String>(&over, &Limits::DEFAULT, &mut plenty()),
            Err(refused)
        );
    }

    #[test]
    fn reports_a_map_that_ends_inside_a_key_or_a_value() {
        let ends_at = |offset| WireError::Malformed {
            offset,
            reason: postcard::Error::DeserializeUnexpectedEnd,
        };
        // Two entries declared; the payload ends before the second key,
        // then before the second value.
        assert_eq!(
            decode::<BTreeMap<u8, u8>>(&[2, 1, 2], &Limits::DEFAULT, &mut plenty()),
            Err(ends_at(3))
        );
        assert_eq!(
            decode::<BTreeMap<u8, u8>>(&[2, 1, 2, 3], &Limits::DEFAULT, &mut plenty()),
            Err(ends_at(4))
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn holds_a_string_inside_a_struct_to_a_lowered_limit() {
        assert_eq!(
            decode::<Track<'_>>(
                &TRACK_OCTETS,
                &lowered(LimitKind::LongText, 2),
                &mut plenty()
            ),
            Ok(TRACK)
        );
        // The title's two octets start after the id's two and its length.
        assert_eq!(
            decode::<Track<'_>>(
                &TRACK_OCTETS,
                &lowered(LimitKind::LongText, 1),
                &mut plenty()
            ),
            Err(WireError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::LongText,
                value: 2,
                max: 1,
                offset: 3,
            }))
        );
    }

    /// A type that ignores the error its string reports.
    #[derive(Debug, PartialEq)]
    struct Lenient(Option<String>);

    impl<'de> Deserialize<'de> for Lenient {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            Ok(Self(String::deserialize(deserializer).ok()))
        }
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn a_broken_limit_fails_the_decode_even_when_the_type_ignores_it() {
        let limits = lowered(LimitKind::LongText, 2);
        assert_eq!(
            decode::<Lenient>(&[2, b'o', b'k'], &limits, &mut plenty()),
            Ok(Lenient(Some("ok".to_owned())))
        );
        assert_eq!(
            decode::<Lenient>(&[3, b'b', b'a', b'd'], &limits, &mut plenty()),
            Err(WireError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::LongText,
                value: 3,
                max: 2,
                offset: 1,
            }))
        );
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn a_long_run_of_units_spends_the_budget_instead_of_running_on() {
        // 1,000 units declared in two octets: nothing to read per unit.
        let units = [0xE8, 0x07];
        let mut budget = payload_budget(&units);
        assert_eq!(budget.remaining(), 72);
        assert_eq!(
            decode::<Vec<()>>(&units, &Limits::DEFAULT, &mut budget),
            Err(WireError::Fault(ParseFault::BudgetExceeded { offset: 2 }))
        );
        assert_eq!(budget.remaining(), 0);
        // With steps to spare, the children limit stops it.
        assert_eq!(
            decode_with::<Vec<()>>(&[0xA0, 0x8D, 0x06], &Limits::DEFAULT, 1 << 20),
            (
                Err(WireError::Fault(ParseFault::LimitExceeded {
                    limit: LimitKind::Children,
                    value: 65_537,
                    max: 65_536,
                    offset: 3,
                })),
                65_538
            )
        );
    }

    #[test]
    fn a_budget_for_a_payload_allows_four_steps_an_octet_and_64_more() {
        assert_eq!(payload_budget(&[]).remaining(), 64);
        assert_eq!(payload_budget(&[0; 10]).remaining(), 104);
    }

    // Versions.

    #[test]
    fn a_server_speaks_its_version_and_the_one_before() {
        assert_eq!(ProtocolVersion::CURRENT, ProtocolVersion(1));
        assert_eq!(server_versions(ProtocolVersion(1)), [ProtocolVersion(1)]);
        assert_eq!(
            server_versions(ProtocolVersion(2)),
            [ProtocolVersion(2), ProtocolVersion(1)]
        );
        assert_eq!(
            server_versions(ProtocolVersion(7)),
            [ProtocolVersion(7), ProtocolVersion(6)]
        );
        assert_eq!(ProtocolVersion(0).previous(), None);
        assert_eq!(ProtocolVersion(1).previous(), None);
        assert_eq!(ProtocolVersion(2).previous(), Some(ProtocolVersion(1)));
        assert_eq!(
            ProtocolVersion(u16::MAX).previous(),
            Some(ProtocolVersion(u16::MAX - 1))
        );
    }

    fn versions(numbers: &[u16]) -> Vec<ProtocolVersion> {
        numbers.iter().copied().map(ProtocolVersion).collect()
    }

    #[test]
    fn agrees_on_the_newest_version_both_sides_speak() {
        let server = versions(&[5, 4]);
        assert_eq!(
            negotiate(&versions(&[4, 5]), &server),
            Ok(ProtocolVersion(5))
        );
        assert_eq!(
            negotiate(&versions(&[6, 4, 3]), &server),
            Ok(ProtocolVersion(4))
        );
        assert_eq!(
            negotiate(&versions(&[3, 4]), &server),
            Ok(ProtocolVersion(4))
        );
    }

    #[test]
    fn tells_an_older_client_to_update() {
        let server = versions(&[5, 4]);
        let update = Err(Upgrade::Client {
            server_newest: Some(ProtocolVersion(5)),
        });
        assert_eq!(negotiate(&versions(&[3, 2]), &server), update);
        assert_eq!(negotiate(&[], &server), update);
        assert_eq!(
            negotiate(&[], &[]),
            Err(Upgrade::Client {
                server_newest: None
            })
        );
    }

    #[test]
    fn tells_a_newer_client_that_the_server_must_update() {
        let server = versions(&[5, 4]);
        assert_eq!(
            negotiate(&versions(&[7, 6]), &server),
            Err(Upgrade::Server {
                client_newest: ProtocolVersion(7)
            })
        );
        assert_eq!(
            negotiate(&versions(&[3, 6]), &server),
            Err(Upgrade::Server {
                client_newest: ProtocolVersion(6)
            })
        );
        assert_eq!(
            negotiate(&versions(&[1]), &[]),
            Err(Upgrade::Server {
                client_newest: ProtocolVersion(1)
            })
        );
    }

    proptest! {
        /// Verifies: SEC-MED-001
        #[test]
        fn returns_for_any_input_on_a_small_stack(
            header in proptest::option::of((any::<u16>(), 1_u8..=5)),
            octets in vec(any::<u8>(), 0..300),
        ) {
            // Any octets at all, or the same octets as the payload of one
            // whole frame, which arbitrary octets almost never are.
            let mut input = Vec::new();
            if let Some((version, kind)) = header {
                let len = u32::try_from(octets.len() + 3).unwrap_or(u32::MAX);
                input.extend(len.to_le_bytes());
                input.extend(version.to_le_bytes());
                input.push(kind);
            }
            input.extend(octets);
            let input_len = input.len();
            let (frame_len, tree, everything) = on_small_stack(move || {
                // A frame that is refused took none of the input.
                let frame_len = read_frame(&input, MAX_FRAME, &mut plenty())
                    .map_or(0, |frame| frame.len);
                let tree = decode_with::<Tree>(&input, &Limits::DEFAULT, 1 << 40).1;
                let everything = decode_with::<Everything<'_>>(&input, &Limits::DEFAULT, 1 << 40).1;
                (frame_len, tree, everything)
            });
            prop_assert!(frame_len <= input_len);
            prop_assert!(header.is_none() || frame_len == input_len);
            // The depth limit stops a tree at its 33rd level.
            prop_assert!(tree <= 33);
            // A type with no unbounded run of zero-octet values stays inside
            // the documented step bound even with steps to spare.
            prop_assert!(everything <= 4 * input_len as u64 + 64);
        }

        /// Any tree deeper than the limit is refused at its 33rd level,
        /// however deep it goes on.
        ///
        /// Verifies: SEC-MED-005, SEC-TM-032
        #[test]
        fn refuses_every_tree_deeper_than_the_limit_on_a_small_stack(nodes in 31_usize..3_000) {
            let decoded = on_small_stack(move || {
                decode_with::<Tree>(&tree_octets(nodes), &Limits::DEFAULT, 1 << 40)
            });
            prop_assert_eq!(
                decoded,
                (
                    Err(WireError::Fault(ParseFault::TooDeep {
                        limit: LimitKind::ContainerDepth,
                        depth: 33,
                        max: 32,
                        offset: 32,
                    })),
                    33
                )
            );
        }

        /// Verifies: SEC-MED-008
        #[test]
        fn every_frame_read_from_a_stream_moves_at_least_seven_octets(
            frames in vec((any::<u16>(), 1_u8..=5, vec(any::<u8>(), 0..40)), 1..6),
            tail in vec(any::<u8>(), 0..40),
        ) {
            // Frames built here octet by octet, then octets that are
            // anything at all.
            let mut stream = Vec::new();
            for (version, kind, payload) in &frames {
                let len = u32::try_from(payload.len() + 3).unwrap_or(u32::MAX);
                stream.extend(len.to_le_bytes());
                stream.extend(version.to_le_bytes());
                stream.push(*kind);
                stream.extend(payload);
            }
            stream.extend(&tail);
            let read = read_all(&stream);
            let whole: Vec<_> = read
                .iter()
                .flatten()
                .take(frames.len())
                .map(|frame| (frame.version.0, frame.kind.octet(), frame.payload.to_vec()))
                .collect();
            prop_assert_eq!(whole, frames);
            // Whatever the tail held, every frame read from it moved on by
            // its header at least, and by no more than the stream holds.
            let lens: Vec<usize> = read.iter().flatten().map(|frame| frame.len).collect();
            prop_assert!(lens.iter().all(|&len| len >= 7));
            prop_assert!(lens.iter().sum::<usize>() <= stream.len());
        }

        #[test]
        fn reads_back_what_it_wrote(
            version in any::<u16>(),
            octet in 1_u8..=5,
            octets in vec(any::<u8>(), 0..300),
        ) {
            let kind = FrameKind::from_octet(octet).unwrap_or(FrameKind::Request);
            let written = write_frame(ProtocolVersion(version), kind, &Buf(octets.clone()))
                .expect("a small payload is written");
            let frame = read_frame(&written, MAX_FRAME, &mut plenty())
                .expect("a written frame reads back");
            prop_assert_eq!(frame.version, ProtocolVersion(version));
            prop_assert_eq!(frame.kind, kind);
            prop_assert_eq!(frame.len, written.len());
            prop_assert_eq!(frame.payload, &written[7..]);
            prop_assert_eq!(
                decode::<Buf>(frame.payload, &Limits::DEFAULT, &mut payload_budget(frame.payload)),
                Ok(Buf(octets))
            );
        }

        /// Repetitive input decoded as lists of units, which take no
        /// octets each: the payload budget lets a decode finish exactly
        /// when it needs no more than 4 steps an octet plus 64, and
        /// otherwise stops it with the budget error.
        ///
        /// Verifies: SEC-MED-007
        #[test]
        fn the_payload_budget_stops_exactly_the_decodes_past_the_step_bound(
            octet in any::<u8>(),
            len in 0_usize..200,
        ) {
            let input = filled(octet, len);
            let bound = 4 * len as u64 + 64;
            let (free, spent) = decode_with::<Vec<Vec<()>>>(&input, &Limits::DEFAULT, 1 << 40);
            let mut budget = payload_budget(&input);
            let capped = decode::<Vec<Vec<()>>>(&input, &Limits::DEFAULT, &mut budget);
            if spent <= bound {
                prop_assert_eq!(capped, free);
                prop_assert_eq!(budget.remaining(), bound - spent);
            } else {
                let out_of_steps = matches!(
                    capped,
                    Err(WireError::Fault(ParseFault::BudgetExceeded { .. }))
                );
                prop_assert!(out_of_steps);
                prop_assert_eq!(budget.remaining(), 0);
            }
        }

        #[test]
        fn agrees_with_a_reference_negotiation(
            client in vec(1_u16..12, 0..5),
            server in vec(1_u16..12, 0..3),
        ) {
            let shared = client.iter().filter(|version| server.contains(version)).max();
            let expected = match (shared, client.iter().max(), server.iter().max()) {
                (Some(&version), _, _) => Ok(ProtocolVersion(version)),
                (None, Some(&newest), None) => Err(Upgrade::Server {
                    client_newest: ProtocolVersion(newest),
                }),
                (None, Some(&newest), Some(&server_newest)) if newest > server_newest => {
                    Err(Upgrade::Server {
                        client_newest: ProtocolVersion(newest),
                    })
                }
                (None, _, server_newest) => Err(Upgrade::Client {
                    server_newest: server_newest.copied().map(ProtocolVersion),
                }),
            };
            prop_assert_eq!(negotiate(&versions(&client), &versions(&server)), expected);
        }
    }
}
