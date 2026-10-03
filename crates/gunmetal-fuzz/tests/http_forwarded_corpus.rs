//! Replays the committed forwarding-header fuzz corpus through its harness
//! on stable Rust, so that `cargo test` and the gate run every seed and
//! every reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/http_forwarded` is a header section, one field
//! per line, and has a test here that pins its exact bytes and the exact
//! outcome the harness reports for it from four peers: loopback and a LAN
//! address with no trusted proxy, and a public proxy and a private overlay
//! proxy. The seeds named after an advisory rebuild the forwarding-header
//! attacks of the rival ledger in `docs/security/rival-security-history.md`.
//! Commit a fuzzing reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::net::IpAddr;
use std::path::PathBuf;

use gunmetal_core::client_context::PathClass::{self, Home, Internet, Loopback};
use gunmetal_core::http::forwarded::{ChainHeader, ForwardedError};
use gunmetal_fuzz::http_forwarded::{Outcome, Resolved, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/http_forwarded")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 10] = [
    "both-chains",
    "cleanuparr-cve-2026-44183-leftmost-local",
    "emby-cve-2023-33193-spoofed-local",
    "empty",
    "ipv6-with-a-port",
    "jellyfin-cve-2025-32012-spoofed-lan",
    "malformed-chain",
    "navidrome-ghsa-f295-6wp9-qqfg-rotating-headers",
    "obfuscated-hop",
    "x-forwarded-for-chain",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(run(&file), expected, "seed {name}");
}

/// A resolved request from `addr`.
fn context(addr: &str, class: PathClass, via_proxy: bool) -> (IpAddr, PathClass, bool) {
    (addr.parse().unwrap(), class, via_proxy)
}

/// The outcome when the section holds forwarding headers: both direct
/// peers keep their own address in the internet class, and both proxied
/// peers resolve as given.
fn ignored_directly(public_proxy: Resolved, overlay_proxy: Resolved) -> Outcome {
    Outcome {
        loopback: Ok(context("127.0.0.1", Internet, false)),
        lan: Ok(context("192.168.1.20", Internet, false)),
        public_proxy,
        overlay_proxy,
    }
}

/// Verifies: SEC-MED-028, SEC-MED-030, SEC-HIS-036
#[test]
fn the_corpus_holds_exactly_the_seeds_tested_here() {
    let mut names: Vec<String> = fs::read_dir(seeds_dir())
        .expect("seed directory is readable")
        .map(|entry| {
            entry
                .expect("seed directory entry is readable")
                .file_name()
                .into_string()
                .expect("seed names are UTF-8")
        })
        .collect();
    names.sort();
    assert_eq!(names, SEEDS);
}

/// With no forwarding header, every peer is classified by its own address.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_the_empty_input() {
    replay(
        "empty",
        b"",
        Outcome {
            loopback: Ok(context("127.0.0.1", Loopback, false)),
            lan: Ok(context("192.168.1.20", Home, false)),
            public_proxy: Ok(context("10.0.0.1", Internet, true)),
            overlay_proxy: Ok(context("fd00::1", Home, true)),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-NET-018
#[test]
fn replays_a_chain_through_two_proxies() {
    replay(
        "x-forwarded-for-chain",
        b"X-Forwarded-For: 203.0.113.7, 10.0.0.2",
        ignored_directly(
            Ok(context("203.0.113.7", Internet, true)),
            Ok(context("203.0.113.7", Internet, true)),
        ),
    );
}

/// Verifies: SEC-MED-028, SEC-NET-018
#[test]
fn replays_an_ipv6_client_with_a_port() {
    replay(
        "ipv6-with-a-port",
        b"Forwarded: for=\"[2001:db8:cafe::17]:4711\";proto=https",
        ignored_directly(
            Ok(context("2001:db8:cafe::17", Internet, true)),
            Ok(context("2001:db8:cafe::17", Home, true)),
        ),
    );
}

/// Verifies: SEC-MED-028, SEC-HIS-003
#[test]
fn replays_an_obfuscated_hop() {
    replay(
        "obfuscated-hop",
        b"Forwarded: for=_gazonk, for=10.0.0.2",
        ignored_directly(
            Ok(context("10.0.0.2", Internet, true)),
            Ok(context("10.0.0.2", Internet, true)),
        ),
    );
}

/// Verifies: SEC-MED-028, SEC-NET-018
#[test]
fn replays_both_chain_headers() {
    replay(
        "both-chains",
        b"Forwarded: for=192.0.2.60\nX-Forwarded-For: 192.0.2.60",
        ignored_directly(
            Err(ForwardedError::BothChains),
            Err(ForwardedError::BothChains),
        ),
    );
}

/// Verifies: SEC-MED-028, SEC-NET-018
#[test]
fn replays_a_malformed_chain() {
    let malformed = ForwardedError::Malformed {
        header: ChainHeader::XForwardedFor,
        offset: 0,
    };
    replay(
        "malformed-chain",
        b"X-Forwarded-For: 203.0.113.7 198.51.100.9",
        ignored_directly(Err(malformed), Err(malformed)),
    );
}

/// Emby, CVE-2023-33193: a forwarded loopback address made a remote request
/// local. Here it is ignored from any peer that is not a trusted proxy, and
/// through one it is only an address, never the loopback class.
///
/// Verifies: SEC-MED-028, SEC-HIS-036, SEC-NET-016
#[test]
fn replays_the_spoofed_local_address_of_emby_cve_2023_33193() {
    replay(
        "emby-cve-2023-33193-spoofed-local",
        b"X-Forwarded-For: 127.0.0.1",
        ignored_directly(
            Ok(context("127.0.0.1", Internet, true)),
            Ok(context("127.0.0.1", Home, true)),
        ),
    );
}

/// Jellyfin, CVE-2025-32012: `X-Forwarded-For` was believed with an empty
/// trusted-proxy list, so a remote request passed as LAN.
///
/// Verifies: SEC-MED-028, SEC-HIS-036, SEC-NET-016
#[test]
fn replays_the_spoofed_lan_address_of_jellyfin_cve_2025_32012() {
    replay(
        "jellyfin-cve-2025-32012-spoofed-lan",
        b"X-Forwarded-For: 192.168.1.10",
        ignored_directly(
            Ok(context("192.168.1.10", Internet, true)),
            Ok(context("192.168.1.10", Home, true)),
        ),
    );
}

/// Navidrome, GHSA-f295-6wp9-qqfg: rotating `X-Forwarded-For`, `X-Real-IP`
/// and `True-Client-IP` dodged the sign-in limiter. Here none of them
/// changes a direct peer's address, and through a proxy only the chain is
/// read.
///
/// Verifies: SEC-MED-028, SEC-HIS-036, SEC-NET-016
#[test]
fn replays_the_rotating_headers_of_navidrome_ghsa_f295_6wp9_qqfg() {
    replay(
        "navidrome-ghsa-f295-6wp9-qqfg-rotating-headers",
        b"X-Forwarded-For: 198.51.100.1\nX-Real-IP: 198.51.100.2\nTrue-Client-IP: 198.51.100.3",
        ignored_directly(
            Ok(context("198.51.100.1", Internet, true)),
            Ok(context("198.51.100.1", Home, true)),
        ),
    );
}

/// Cleanuparr, CVE-2026-44183: the leftmost, client-written entry was
/// taken as the client. Here the walk from the right stops at the first
/// untrusted hop.
///
/// Verifies: SEC-MED-028, SEC-HIS-036, SEC-HIS-003
#[test]
fn replays_the_leftmost_local_address_of_cleanuparr_cve_2026_44183() {
    replay(
        "cleanuparr-cve-2026-44183-leftmost-local",
        b"X-Forwarded-For: 127.0.0.1, 203.0.113.7",
        ignored_directly(
            Ok(context("203.0.113.7", Internet, true)),
            Ok(context("203.0.113.7", Home, true)),
        ),
    );
}
