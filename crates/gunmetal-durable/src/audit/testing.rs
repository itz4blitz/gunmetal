//! What the audit log's tests share: a data root, MAC fakes, and a counted
//! salt source.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use gunmetal_core::http::forwarded::{ForwardingHeaders, HostNetwork, path_class};
use gunmetal_core::id::{IdKind, PublicId};
use gunmetal_core::token::mac::MacProvider;
use gunmetal_fs::dataroot::{DataRoot, Policy};
use gunmetal_fs::host::HostFacts;
use gunmetal_secrets::random::{Random, RandomnessUnavailable};
use gunmetal_testkit::tempdir::TempDir;

use super::log::{AuditLog, Limits};
use super::record::WriteClass;

/// A temporary data directory and its open root.
pub(crate) struct Data {
    pub(crate) dir: TempDir,
    pub(crate) root: DataRoot,
}

/// A fresh data root on real files.
pub(crate) fn data() -> Data {
    let dir = TempDir::new("audit").expect("a temporary directory");
    let host = HostFacts::probe(dir.path()).expect("the host is probed");
    let root = DataRoot::open(dir.path(), &host, Policy::DEFAULT)
        .expect("the data root opens")
        .root;
    Data { dir, root }
}

/// A second handle on `data`'s directory, so the log can own its root.
pub(crate) fn handle(data: &Data) -> DataRoot {
    let host = HostFacts::probe(data.dir.path()).expect("the host is probed");
    DataRoot::open(data.dir.path(), &host, Policy::DEFAULT)
        .expect("the data root opens")
        .root
}

/// HMAC-SHA-256 stand-in whose tag depends on the key id and the message.
pub(crate) struct MixMac {
    current: u8,
}

impl MixMac {
    pub(crate) fn new(current: u8) -> Self {
        Self { current }
    }
}

impl MacProvider for MixMac {
    fn current_kid(&self) -> u8 {
        self.current
    }

    fn mac(&self, kid: u8, msg: &[u8]) -> Option<[u8; 32]> {
        (kid == self.current).then_some(mix(kid, msg))
    }
}

/// A MAC that never answers.
pub(crate) struct FailingMac;

impl MacProvider for FailingMac {
    fn current_kid(&self) -> u8 {
        0
    }

    fn mac(&self, _: u8, _: &[u8]) -> Option<[u8; 32]> {
        None
    }
}

/// Independent mix, written only for tests: a tagged 32-byte digest of
/// `kid` and `msg`.
pub(crate) fn mix(kid: u8, msg: &[u8]) -> [u8; 32] {
    let mut a: u64 = 0x9E37_79B9_7F4A_7C15 ^ u64::from(kid);
    let mut b: u64 = 0xBF58_476D_1CE4_E5B9;
    let mut c: u64 = 0x94D0_49BB_1331_11EB;
    let mut d: u64 = 0xC2B2_AE3D_27D4_EB4F;
    for &octet in msg {
        a = a.rotate_left(7) ^ u64::from(octet).wrapping_mul(0xD6E8_FEB8_66D2_2BE0);
        b = b.wrapping_add(u64::from(octet)).rotate_left(11) ^ a;
        c = c.wrapping_mul(0xA24B_AED4_96E9_C13F) ^ b;
        d = d.rotate_left(u32::from(octet % 63).saturating_add(1)) ^ u64::from(octet);
    }
    d ^= u64::try_from(msg.len()).unwrap_or(u64::MAX);
    let mut out = [0_u8; 32];
    let words = [
        a.to_be_bytes(),
        b.to_be_bytes(),
        c.to_be_bytes(),
        d.to_be_bytes(),
    ];
    for (chunk, word) in out.chunks_mut(8).zip(words) {
        chunk.copy_from_slice(&word);
    }
    out
}

/// A CSPRNG stand-in that writes 0, 1, 2, … so salts are known.
pub(crate) struct Counted {
    next: AtomicU64,
}

impl Counted {
    pub(crate) fn new() -> Self {
        Self {
            next: AtomicU64::new(0),
        }
    }
}

impl Random for Counted {
    fn fill(&self, bytes: &mut [u8]) -> Result<(), RandomnessUnavailable> {
        for slot in bytes.iter_mut() {
            let n = self.next.fetch_add(1, Ordering::SeqCst);
            *slot = u8::try_from(n & 0xFF).unwrap_or(0);
        }
        Ok(())
    }
}

/// A source that never supplies bytes.
pub(crate) struct FailingRandom;

impl Random for FailingRandom {
    fn fill(&self, _: &mut [u8]) -> Result<(), RandomnessUnavailable> {
        Err(RandomnessUnavailable)
    }
}

/// The canonical test user.
pub(crate) fn account() -> PublicId {
    PublicId::parse("usr_0123456789abcdefghjkmnpqrs", IdKind::User).expect("a canonical user ID")
}

/// 203.0.113.7, internet, not via a proxy.
pub(crate) fn internet() -> gunmetal_core::client_context::ClientContext {
    arriving(Ipv4Addr::new(203, 0, 113, 7))
}

/// A loopback request.
pub(crate) fn loopback() -> gunmetal_core::client_context::ClientContext {
    arriving(Ipv4Addr::LOCALHOST)
}

fn arriving(peer: Ipv4Addr) -> gunmetal_core::client_context::ClientContext {
    path_class(
        IpAddr::V4(peer),
        &ForwardingHeaders::default(),
        &[],
        &HostNetwork::default(),
    )
    .expect("a direct peer has a path class")
}

/// An IPv6 documentation address.
pub(crate) fn internet_v6() -> gunmetal_core::client_context::ClientContext {
    path_class(
        IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
        &ForwardingHeaders::default(),
        &[],
        &HostNetwork::default(),
    )
    .expect("a direct peer has a path class")
}

/// An open log with small segments so rotation is tested.
pub(crate) fn log(data: &Data) -> AuditLog {
    limited(data, Limits::test())
}

pub(crate) fn limited(data: &Data, limits: Limits) -> AuditLog {
    AuditLog::open_with(
        handle(data),
        Arc::new(MixMac::new(0)),
        Arc::new(MixMac::new(1)),
        Arc::new(Counted::new()),
        limits,
    )
    .expect("the audit log opens")
}

/// Shared lock so tests that mutate files do not race themselves.
pub(crate) const ORDINARY: WriteClass = WriteClass::Ordinary;
