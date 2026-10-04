//! Replays the committed link fuzz corpus through its harness on stable
//! Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/link` has a test here that pins its exact bytes
//! and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::link::LinkError;
use gunmetal_fuzz::link::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/link")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 11] = [
    "credentials-before-the-host",
    "empty",
    "host-with-a-quote",
    "host-with-an-escaped-quote",
    "https-with-a-port",
    "invalid-utf8-in-the-host",
    "ipv4-in-hexadecimal",
    "known-route",
    "scheme-relative-target",
    "scheme-split-by-a-tab",
    "tail-with-a-quote-and-a-backslash",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

/// A value that is not a link and is not a known route.
fn refused(error: LinkError) -> Outcome {
    Outcome {
        link: Err(error),
        return_target: "/".to_owned(),
    }
}

/// A link that opens `href` and shows `host`, and is not a known route.
fn opens(href: &str, host: &str) -> Outcome {
    Outcome {
        link: Ok((href.to_owned(), host.to_owned())),
        return_target: "/".to_owned(),
    }
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
    replay("empty", &[], &refused(LinkError::NotAbsolute));
}

/// The scheme and host are lower-cased, the leading zero of the port is
/// dropped, and the path, query and fragment are kept.
///
/// Verifies: SEC-MED-028, SEC-API-047, SEC-STD-015
#[test]
fn replays_an_https_link_with_a_port() {
    replay(
        "https-with-a-port",
        b"HTTPS://Example.COM:08443/a?b#c",
        &opens("https://example.com:8443/a?b#c", "example.com"),
    );
}

/// A browser strips the leading space and the tab and runs the script, so
/// the validator reads the scheme the same way and refuses it.
///
/// Verifies: SEC-MED-028, SEC-API-047, SEC-CLI-002
#[test]
fn replays_a_scheme_split_by_a_tab() {
    replay(
        "scheme-split-by-a-tab",
        b" Java\tScript:alert(1)",
        &refused(LinkError::NotHttps),
    );
}

/// The link names `bank.example` but goes to `evil.example`.
///
/// Verifies: SEC-MED-028, SEC-API-047
#[test]
fn replays_credentials_before_the_host() {
    replay(
        "credentials-before-the-host",
        b"https://bank.example@evil.example/",
        &refused(LinkError::Credentials),
    );
}

/// The host is shown as the dotted address the browser will open.
///
/// Verifies: SEC-MED-028, SEC-API-047, SEC-STD-015
#[test]
fn replays_an_ipv4_host_in_hexadecimal() {
    replay(
        "ipv4-in-hexadecimal",
        b"https://0xC0.0.2.1/",
        &opens("https://192.0.2.1/", "192.0.2.1"),
    );
}

/// The invalid octet becomes U+FFFD, which no host may hold.
///
/// Verifies: SEC-MED-028, SEC-API-047
#[test]
fn replays_invalid_utf8_in_the_host() {
    replay(
        "invalid-utf8-in-the-host",
        b"https://ex\xFFample.com",
        &refused(LinkError::BadHost),
    );
}

/// A domain holds only ASCII letters, digits, `-`, `.` and `_`, so a host
/// that would end the attribute the link is placed in stays plain text.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_host_with_a_quote() {
    replay(
        "host-with-a-quote",
        b"https://x\"onclick=alert(1)\"/",
        &refused(LinkError::BadHost),
    );
}

/// The escape is decoded before the host is checked, so the apostrophe it
/// names is refused as a written one is.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_host_with_an_escaped_quote() {
    replay(
        "host-with-an-escaped-quote",
        b"https://ex%27ample.com/",
        &refused(LinkError::BadHost),
    );
}

/// Verifies: SEC-MED-028, SEC-API-070
#[test]
fn replays_a_known_route() {
    replay(
        "known-route",
        b"/albums/a1-b_2",
        &Outcome {
            link: Err(LinkError::NotAbsolute),
            return_target: "/albums/a1-b_2".to_owned(),
        },
    );
}

/// Read as a return target, `//evil.example` would leave the site, so it
/// goes home.
///
/// Verifies: SEC-MED-028, SEC-API-070, SEC-HIS-032
#[test]
fn replays_a_scheme_relative_target() {
    replay(
        "scheme-relative-target",
        b"//evil.example/library",
        &refused(LinkError::NotAbsolute),
    );
}

/// A backslash in the path is the slash a browser reads it as; a space, a
/// quote, a backslash in the query and a letter outside ASCII are written
/// as percent escapes, so the link cannot end an attribute it is placed in.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_tail_with_a_quote_and_a_backslash() {
    replay(
        "tail-with-a-quote-and-a-backslash",
        b"https://example.com/a\\b \"c\"?d\\e#caf\xC3\xA9",
        &opens(
            "https://example.com/a/b%20%22c%22?d%5Ce#caf%C3%A9",
            "example.com",
        ),
    );
}
