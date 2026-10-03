//! Replays the committed network fuzz corpus through its harness on stable
//! Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/net` has a test here that pins its exact bytes
//! and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.

use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::PathBuf;

use gunmetal_core::net::{AddrClass, NetError};
use gunmetal_fuzz::net::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/net")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 7] = [
    "empty",
    "host-bits-set",
    "ipv4-mapped-loopback",
    "leading-zero-in-the-prefix",
    "nat64-of-a-private-address",
    "shared-address-space",
    "tailnet-range",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(run(&file), expected, "seed {name}");
}

/// Verifies: SEC-MED-028, SEC-MED-030
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

/// Verifies: SEC-MED-028
#[test]
fn replays_the_empty_input() {
    replay(
        "empty",
        &[],
        Outcome {
            network: Err(NetError::NoPrefix),
            class: None,
        },
    );
}

/// Tailscale's IPv6 range, inside unique-local `fc00::/7`.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_the_tailnet_range() {
    replay(
        "tailnet-range",
        b"fd7a:115c:a1e0::/48",
        Outcome {
            network: Ok((
                IpAddr::V6(Ipv6Addr::new(0xfd7a, 0x115c, 0xa1e0, 0, 0, 0, 0, 0)),
                48,
            )),
            class: Some(AddrClass::Private),
        },
    );
}

/// RFC 6598's shared address space, `100.64.0.0/10`.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_the_shared_address_space() {
    replay(
        "shared-address-space",
        b"100.64.0.0/10",
        Outcome {
            network: Ok((IpAddr::V4(Ipv4Addr::new(100, 64, 0, 0)), 10)),
            class: Some(AddrClass::SharedAddressSpace),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_host_written_as_a_network() {
    replay(
        "host-bits-set",
        b"192.168.1.1/24",
        Outcome {
            network: Err(NetError::HostBitsSet),
            class: Some(AddrClass::Private),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_prefix_with_a_leading_zero() {
    replay(
        "leading-zero-in-the-prefix",
        b"10.0.0.0/08",
        Outcome {
            network: Err(NetError::BadPrefix),
            class: Some(AddrClass::Private),
        },
    );
}

/// `::ffff:127.0.0.1` is loopback once it is read as the IPv4 address it
/// maps.
///
/// Verifies: SEC-MED-028, SEC-NET-025
#[test]
fn replays_an_ipv4_mapped_loopback_address() {
    replay(
        "ipv4-mapped-loopback",
        b"::ffff:127.0.0.1/128",
        Outcome {
            network: Ok((
                IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0x7f00, 1)),
                128,
            )),
            class: Some(AddrClass::Loopback),
        },
    );
}

/// `64:ff9b::a00:1` carries the private address 10.0.0.1 through NAT64,
/// and is never local, whatever it carries.
///
/// Verifies: SEC-MED-028, SEC-NET-025
#[test]
fn replays_nat64_of_a_private_address() {
    replay(
        "nat64-of-a-private-address",
        b"64:ff9b::a00:1/128",
        Outcome {
            network: Ok((
                IpAddr::V6(Ipv6Addr::new(0x64, 0xff9b, 0, 0, 0, 0, 0x0a00, 1)),
                128,
            )),
            class: Some(AddrClass::Translated),
        },
    );
}
