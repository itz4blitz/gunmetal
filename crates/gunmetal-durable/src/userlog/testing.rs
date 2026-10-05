//! What the user log's tests share: a data root in a temporary directory,
//! a few events, and the octets of a segment written out independently of
//! the code under test.

use std::io::Read;

use gunmetal_core::userdata::codec;
use gunmetal_core::userdata::event::{
    Body, ContentId, DeviceId, Event, EventId, ItemRef, Play, ProfileId, Stream,
};
use gunmetal_core::userdata::hlc::Hlc;
use gunmetal_fs::dataroot::{DataRoot, Policy};
use gunmetal_fs::host::HostFacts;
use gunmetal_fs::path::{DataPath, LogMonth};
use gunmetal_testkit::tempdir::TempDir;

/// Alice's stream.
pub(crate) const ALICE: Stream = Stream::Profile(ProfileId::new([0xA1; 16]));
/// Bob's stream.
pub(crate) const BOB: Stream = Stream::Profile(ProfileId::new([0xB2; 16]));
/// The device every test event comes from.
pub(crate) const PHONE: DeviceId = DeviceId::new([0xD0; 16]);
/// The item every test event is about.
pub(crate) const SONG: ContentId = ContentId::new([0xC0; 32]);

/// A temporary data directory and its open root.
pub(crate) struct Data {
    /// Kept so the directory lives as long as the root.
    _dir: TempDir,
    /// The data root.
    pub(crate) root: DataRoot,
}

/// A fresh data root on real files.
pub(crate) fn data() -> Data {
    let dir = TempDir::new("userlog").expect("a temporary directory");
    let host = HostFacts::probe(dir.path()).expect("the host is probed");
    let root = DataRoot::open(dir.path(), &host, Policy::DEFAULT)
        .expect("the data root opens")
        .root;
    Data { _dir: dir, root }
}

/// The event with ID `id` repeated, in `stream`, at wall time `wall_ms`.
fn event(id: u8, stream: Stream, wall_ms: u64, body: Body) -> Event {
    Event {
        id: EventId::new([id; 16]),
        clock: Hlc::new(wall_ms, 0),
        device: PHONE,
        stream,
        body,
    }
}

/// A play of [`SONG`] to its end, 30 ms long.
pub(crate) fn play(id: u8, stream: Stream, wall_ms: u64) -> Event {
    let body = Body::Play(Play {
        item: SONG,
        position_ms: 30,
        completed: true,
    });
    event(id, stream, wall_ms, body)
}

/// A love of [`SONG`], which is not history.
pub(crate) fn love(id: u8, stream: Stream, wall_ms: u64) -> Event {
    event(id, stream, wall_ms, Body::Love(ItemRef::Content(SONG)))
}

/// Month `number` of `year`.
pub(crate) fn of(year: u16, number: u8) -> LogMonth {
    LogMonth::new(year, number).expect("a month of the calendar")
}

/// CRC-32C from the unreflected Castagnoli polynomial, written only for
/// these tests: each octet is reflected, bits are shifted out of the top,
/// and the result is reflected.
fn crc32c(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for &byte in bytes {
        crc ^= u32::from(byte.reverse_bits()) << 24;
        for _ in 0..8 {
            crc = if crc & 0x8000_0000 == 0 {
                crc << 1
            } else {
                (crc << 1) ^ 0x1EDC_6F41
            };
        }
    }
    (!crc).reverse_bits()
}

/// `payload` framed as one record of format `version`: the length, the
/// CRC-32C of the version and the payload, the version, the payload.
pub(crate) fn frame_version(version: u8, payload: &[u8]) -> Vec<u8> {
    let mut hashed = vec![version];
    hashed.extend(payload);
    let len = u32::try_from(payload.len()).expect("a payload that fits a record");
    let mut out = len.to_le_bytes().to_vec();
    out.extend(crc32c(&hashed).to_le_bytes());
    out.push(version);
    out.extend(payload);
    out
}

/// `payload` framed as one record of the current format.
pub(crate) fn frame(payload: &[u8]) -> Vec<u8> {
    frame_version(1, payload)
}

/// The header record of the segment of the stream whose identity is `id`
/// for month `number` of `year`.
pub(crate) fn header(id: [u8; 16], year: u16, number: u8) -> Vec<u8> {
    let mut payload = id.to_vec();
    payload.extend(year.to_le_bytes());
    payload.push(number);
    frame(&payload)
}

/// The record that holds `event` as number `seq`.
pub(crate) fn record(seq: u64, event: &Event) -> Vec<u8> {
    let mut payload = seq.to_le_bytes().to_vec();
    payload.extend(codec::encode(event));
    frame(&payload)
}

/// Everything the file at `path` holds, read through the data root.
pub(crate) fn bytes(root: &DataRoot, path: &DataPath) -> Vec<u8> {
    let mut found = Vec::new();
    root.open_read(path)
        .expect("the file opens")
        .read_to_end(&mut found)
        .expect("the file reads");
    found
}

#[test]
fn the_test_framing_matches_the_published_check_value() {
    // CRC-32/ISCSI in the RevEng catalogue.
    assert_eq!(crc32c(b"123456789"), 0xE306_9283);
    // Length 2, CRC-32C of [1, 'h', 'i'], version 1, "hi".
    assert_eq!(
        frame(b"hi"),
        [
            0x02, 0x00, 0x00, 0x00, 0x14, 0x9F, 0xD9, 0xC1, 0x01, b'h', b'i'
        ]
    );
}
