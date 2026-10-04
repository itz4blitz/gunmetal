//! The harness for the wire codec in `gunmetal_core::wire`: frames, the
//! payload decoder and version negotiation (SEC-MED-023, SEC-CLI-021).

use gunmetal_core::parse::{Budget, LimitKind, Limits, ParseFault};
use gunmetal_core::wire::{self, FrameKind, MAX_FRAME, ProtocolVersion, Upgrade, WireError};

/// The frame cap every read uses, small enough that fuzzing reaches it
/// often.
pub const CAP: u32 = 64;

/// The most entries a decoded list may hold here, lowered from the default
/// so that fuzzing reaches it often.
pub const ENTRIES: u64 = 4;

/// The most octets a decoded string may hold here, lowered for the same
/// reason.
pub const TEXT: u64 = 8;

/// The protocol type every payload is decoded as: a number, a string, an
/// optional flag and a list of results, which postcard writes as enums.
pub type Sample = (u16, String, Option<bool>, Vec<Result<u8, String>>);

/// One frame read from the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Read {
    /// The frame's protocol version.
    pub version: ProtocolVersion,
    /// The frame's kind.
    pub kind: FrameKind,
    /// Octets the frame took, length field included.
    pub len: usize,
    /// [`wire::decode`] of the payload as a [`Sample`].
    pub decoded: Result<Sample, WireError>,
}

/// What the codec reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Every frame [`wire::read_frame`] read, one after another.
    pub frames: Vec<Read>,
    /// The error that stopped the reading, or `None` when the frames used
    /// up the input.
    pub stop: Option<WireError>,
    /// [`wire::negotiate`] between the frames' versions, as a client's
    /// offer, and the versions of a server at [`ProtocolVersion::CURRENT`].
    pub agreed: Result<ProtocolVersion, Upgrade>,
}

/// Reads `data` as a stream of frames capped at [`CAP`], decodes each
/// payload as a [`Sample`] under lowered limits and its payload budget,
/// writes each decoded sample back as a frame, and negotiates between the
/// versions the frames carry and the server's.
///
/// # Panics
///
/// Panics when a result breaks an invariant that holds for every input: a
/// frame shorter than its seven-octet header, longer than the input, or
/// whose payload is not the octets after its header; a kind that does not
/// survive its octet; a reading budget of one step per octet running out; a
/// decoded sample past the lowered limits, or one that does not read back
/// from the frame written for it; a server that does not speak its current
/// version and the one before; or a negotiated version that one side does
/// not speak, or an upgrade that names the wrong side.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let limits = Limits::DEFAULT
        .with_override(LimitKind::Children, ENTRIES)
        .and_then(|limits| limits.with_override(LimitKind::LongText, TEXT))
        .unwrap_or(Limits::DEFAULT);
    let mut budget = Budget::for_input(u64::try_from(data.len()).unwrap_or(u64::MAX), 1, 1);
    let mut frames = Vec::new();
    let mut rest = data;
    let stop = loop {
        if rest.is_empty() {
            break None;
        }
        match wire::read_frame(rest, CAP, &mut budget) {
            Err(error) => break Some(error),
            Ok(frame) => {
                let decoded = wire::decode::<Sample>(
                    frame.payload,
                    &limits,
                    &mut wire::payload_budget(frame.payload),
                );
                assert!(
                    frame.len >= 7
                        && rest.get(7..frame.len) == Some(frame.payload)
                        && FrameKind::from_octet(frame.kind.octet()) == Some(frame.kind)
                        && decoded.as_ref().ok().is_none_or(|sample| {
                            sample.1.len() <= 8
                                && sample.3.len() <= 4
                                && sample.3.iter().all(|entry| {
                                    entry.as_ref().err().is_none_or(|text| text.len() <= 8)
                                })
                                && wire::write_frame(frame.version, frame.kind, sample)
                                    .ok()
                                    .and_then(|written| {
                                        wire::read_frame(
                                            &written,
                                            MAX_FRAME,
                                            &mut Budget::for_input(0, 0, 1),
                                        )
                                        .ok()
                                        .map(|again| {
                                            (
                                                again.version,
                                                again.kind,
                                                again.len == written.len(),
                                                wire::decode::<Sample>(
                                                    again.payload,
                                                    &limits,
                                                    &mut wire::payload_budget(again.payload),
                                                ),
                                            )
                                        })
                                    })
                                    == Some((frame.version, frame.kind, true, Ok(sample.clone())))
                        })
                );
                frames.push(Read {
                    version: frame.version,
                    kind: frame.kind,
                    len: frame.len,
                    decoded,
                });
                rest = rest.get(frame.len..).unwrap_or_default();
            }
        }
    };
    let client: Vec<ProtocolVersion> = frames.iter().map(|frame| frame.version).collect();
    let server = wire::server_versions(ProtocolVersion::CURRENT);
    let agreed = wire::negotiate(&client, &server);
    assert!(
        !matches!(
            stop,
            Some(WireError::Fault(ParseFault::BudgetExceeded { .. }))
        ) && server.first() == Some(&ProtocolVersion::CURRENT)
            && server.get(1).copied() == ProtocolVersion::CURRENT.previous()
            && server.len() <= 2
            && match agreed {
                Ok(version) => client.contains(&version) && server.contains(&version),
                Err(Upgrade::Client { server_newest }) => {
                    server_newest == server.iter().max().copied()
                        && client.iter().all(|version| !server.contains(version))
                }
                Err(Upgrade::Server { client_newest }) => {
                    client.contains(&client_newest)
                        && server.iter().all(|version| *version < client_newest)
                }
            }
    );
    Outcome {
        frames,
        stop,
        agreed,
    }
}
