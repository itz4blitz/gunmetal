//! Record types the audit log stores and returns: own-person views, truncated
//! admin views, and signed checkpoint heads.

use std::net::IpAddr;
use std::ops::Range;

use gunmetal_core::client_context::PathClass;
use gunmetal_core::id::PublicId;
use gunmetal_core::time::Timestamp;

/// A sequence number, 1-based.
pub type Seq = u64;

/// A page of records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    /// The records in sequence order.
    pub records: Vec<T>,
}

pub(crate) trait HasSeq {
    fn seq(&self) -> u64;
}

/// The records whose sequence numbers fall in `range`.
#[must_use]
pub(crate) fn in_range<T: HasSeq>(page: Page<T>, range: Range<u64>) -> Page<T> {
    Page {
        records: page
            .records
            .into_iter()
            .filter(|record| range.contains(&record.seq()))
            .collect(),
    }
}

/// How an event ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The action succeeded.
    Success,
    /// The action failed.
    Fail,
    /// The action was refused.
    Denied,
    /// A setting was switched on.
    Enabled,
}

impl Outcome {
    /// The JSON spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Fail => "fail",
            Self::Denied => "denied",
            Self::Enabled => "enabled",
        }
    }

    pub(crate) fn parse(text: &str) -> Option<Self> {
        match text {
            "success" => Some(Self::Success),
            "fail" => Some(Self::Fail),
            "denied" => Some(Self::Denied),
            "enabled" => Some(Self::Enabled),
            _ => None,
        }
    }
}

/// A record as the person it is about sees it: full address while the side
/// store still holds it (SEC-IAM-097).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnRecord {
    /// Sequence number.
    pub seq: Seq,
    /// UTC time.
    pub ts: Timestamp,
    /// Catalogue vocabulary name.
    pub event: String,
    /// The account named on the event, if any.
    pub account: Option<PublicId>,
    /// The source address while it is still stored.
    pub addr: Option<IpAddr>,
    /// The path class, when the event had a source.
    pub class: Option<PathClass>,
    /// How the action ended.
    pub outcome: Outcome,
    /// The record's hash.
    pub hash: [u8; 32],
}

impl HasSeq for OwnRecord {
    fn seq(&self) -> u64 {
        self.seq
    }
}

/// An address truncated so it cannot hold another person's full IP
/// (SEC-OPS-027).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TruncatedAddr {
    /// An IPv4 /24 prefix, such as `203.0.113.0/24`.
    V4Prefix(String),
    /// An IPv6 /48 prefix, such as `2001:db8::/48`.
    V6Prefix(String),
}

/// A record as an audit-capability holder sees it: other people's addresses
/// are truncated in the type itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TruncatedRecord {
    /// Sequence number.
    pub seq: Seq,
    /// UTC time.
    pub ts: Timestamp,
    /// Catalogue vocabulary name.
    pub event: String,
    /// The account named on the event, if any.
    pub account: Option<PublicId>,
    /// The truncated source, when one is still on file.
    pub addr: Option<TruncatedAddr>,
    /// The path class, when the event had a source.
    pub class: Option<PathClass>,
    /// How the action ended.
    pub outcome: Outcome,
    /// The record's hash.
    pub hash: [u8; 32],
}

impl HasSeq for TruncatedRecord {
    fn seq(&self) -> u64 {
        self.seq
    }
}

/// The latest signed checkpoint (SEC-OPS-075).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedHead {
    /// Sequence number of the checkpoint record.
    pub seq: Seq,
    /// Hash of the head at the checkpoint.
    pub head: [u8; 32],
    /// Key identifier that produced `mac`.
    pub kid: u8,
    /// HMAC-SHA-256 of the checkpoint payload.
    pub mac: [u8; 32],
    /// When the checkpoint was written.
    pub at: Timestamp,
}

/// What a retention run did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionReport {
    /// Addresses coarsened.
    pub coarsened: u64,
    /// Addresses removed.
    pub removed: u64,
    /// Whole event records dropped with their segments.
    pub pruned: u64,
}

/// Whether an append may spend the disk reserve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteClass {
    /// An ordinary security event: refused when the disk is full.
    Ordinary,
    /// A recovery action (retention, host CLI): uses the reserve.
    Recovery,
}

/// Path class as a JSON token.
pub(crate) fn class_name(class: PathClass) -> &'static str {
    match class {
        PathClass::Loopback => "loopback",
        PathClass::Home => "home",
        PathClass::Unknown => "unknown",
        PathClass::Internet => "internet",
        PathClass::UntrustedProxy => "untrusted_proxy",
    }
}

pub(crate) fn parse_class(text: &str) -> Option<PathClass> {
    match text {
        "loopback" => Some(PathClass::Loopback),
        "home" => Some(PathClass::Home),
        "unknown" => Some(PathClass::Unknown),
        "internet" => Some(PathClass::Internet),
        "untrusted_proxy" => Some(PathClass::UntrustedProxy),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Outcome, OwnRecord, Page, TruncatedAddr, TruncatedRecord, class_name, in_range, parse_class,
    };
    use gunmetal_core::client_context::PathClass;
    use gunmetal_core::time::Timestamp;
    use std::net::{IpAddr, Ipv4Addr};

    fn rec(seq: u64) -> TruncatedRecord {
        TruncatedRecord {
            seq,
            ts: Timestamp::from_millis(0).expect("epoch"),
            event: "gm_egress_denied".to_owned(),
            account: None,
            addr: None,
            class: None,
            outcome: Outcome::Denied,
            hash: [0; 32],
        }
    }

    #[test]
    fn names_every_path_class_and_outcome() {
        let classes = [
            PathClass::Loopback,
            PathClass::Home,
            PathClass::Unknown,
            PathClass::Internet,
            PathClass::UntrustedProxy,
        ];
        let names: Vec<&str> = classes.iter().copied().map(class_name).collect();
        assert_eq!(
            names,
            ["loopback", "home", "unknown", "internet", "untrusted_proxy"]
        );
        let parsed: Vec<Option<PathClass>> = names.iter().copied().map(parse_class).collect();
        assert_eq!(parsed, classes.into_iter().map(Some).collect::<Vec<_>>());
        assert_eq!(parse_class("lan"), None);
        assert_eq!(
            [
                Outcome::Success,
                Outcome::Fail,
                Outcome::Denied,
                Outcome::Enabled
            ]
            .map(Outcome::as_str),
            ["success", "fail", "denied", "enabled"]
        );
        assert_eq!(Outcome::parse("success"), Some(Outcome::Success));
        assert_eq!(Outcome::parse("fail"), Some(Outcome::Fail));
        assert_eq!(Outcome::parse("denied"), Some(Outcome::Denied));
        assert_eq!(Outcome::parse("enabled"), Some(Outcome::Enabled));
        assert_eq!(Outcome::parse("nope"), None);
        assert_eq!(
            TruncatedAddr::V4Prefix("203.0.113.0/24".to_owned()),
            TruncatedAddr::V4Prefix("203.0.113.0/24".to_owned())
        );
    }

    #[test]
    fn a_page_keeps_only_the_asked_range() {
        let page = Page {
            records: vec![rec(1), rec(2), rec(3)],
        };
        assert_eq!(
            in_range(page, 2..4)
                .records
                .iter()
                .map(|r| r.seq)
                .collect::<Vec<_>>(),
            [2, 3]
        );
        let own = Page {
            records: vec![own_rec(1), own_rec(2), own_rec(3)],
        };
        assert_eq!(
            in_range(own, 2..4)
                .records
                .iter()
                .map(|r| r.seq)
                .collect::<Vec<_>>(),
            [2, 3]
        );
    }

    fn own_rec(seq: u64) -> OwnRecord {
        OwnRecord {
            seq,
            ts: Timestamp::from_millis(0).expect("epoch"),
            event: "gm_egress_denied".to_owned(),
            account: None,
            addr: Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
            class: None,
            outcome: Outcome::Denied,
            hash: [0; 32],
        }
    }
}
