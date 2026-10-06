//! Replays the committed inbound-link fuzz corpus through its harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/deeplink` has a test here that pins its exact
//! bytes and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_fuzz::deeplink::{Outcome, Seen, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/deeplink")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 15] = [
    "claim",
    "claim-code-in-the-query",
    "claim-on-this-machine",
    "credentials-before-the-host",
    "empty",
    "invalid-utf8-in-the-secret",
    "invitation",
    "lookalike-host",
    "lower-case-claim-code",
    "padded-secret",
    "pairing",
    "plain-http-to-another-machine",
    "recovery",
    "server-spelled-another-way",
    "unknown-path",
];

/// The octets 0 to 15, which a link writes as `AAECAwQFBgcICQoLDA0ODw`.
const SECRET: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

/// A route that was read, which asks for a confirmation screen.
fn read(route: Seen) -> Outcome {
    Outcome {
        route,
        needs_confirmation: true,
    }
}

/// Text that is not a link, which asks for nothing.
fn not_recognised() -> Outcome {
    Outcome {
        route: Seen::NotRecognised,
        needs_confirmation: false,
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

/// Verifies: SEC-MED-028, SEC-CLI-025
#[test]
fn replays_the_empty_input() {
    replay("empty", &[], &not_recognised());
}

/// Verifies: SEC-MED-028, SEC-CLI-025
#[test]
fn replays_a_claim_link() {
    replay(
        "claim",
        b"https://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
        &read(Seen::Claim {
            server: "https://music.example".to_owned(),
            code: "01234-56789-ABCDE-FGHJK-MNPQR-S9".to_owned(),
        }),
    );
}

/// A server is claimed before it has a name, from the machine it runs on.
///
/// Verifies: SEC-MED-028, SEC-CLI-025
#[test]
fn replays_a_claim_link_on_this_machine() {
    replay(
        "claim-on-this-machine",
        b"http://localhost:8096/claim#00000-00000-00000-00000-00000-00",
        &read(Seen::Claim {
            server: "http://localhost:8096".to_owned(),
            code: "00000-00000-00000-00000-00000-00".to_owned(),
        }),
    );
}

/// Verifies: SEC-MED-028, SEC-CLI-025
#[test]
fn replays_an_invitation_link() {
    replay(
        "invitation",
        b"https://music.example/invite#AAECAwQFBgcICQoLDA0ODw",
        &read(Seen::Invitation {
            server: "https://music.example".to_owned(),
            secret: SECRET,
        }),
    );
}

/// The pairing code, then the server's identity key: the octets 0 to 31.
///
/// Verifies: SEC-MED-028, SEC-CLI-025
#[test]
fn replays_a_pairing_link() {
    replay(
        "pairing",
        b"https://music.example:8443/pair#WDJB-MJHT.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
        &read(Seen::Pairing {
            server: "https://music.example:8443".to_owned(),
            code: "WDJB-MJHT".to_owned(),
            key: [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23, 24, 25, 26, 27, 28, 29, 30, 31,
            ],
        }),
    );
}

/// The server is an address in brackets, and the secret is 16 octets of
/// 255, which URL-safe base64 writes with underscores.
///
/// Verifies: SEC-MED-028, SEC-CLI-025
#[test]
fn replays_a_recovery_link() {
    replay(
        "recovery",
        b"https://[2001:db8::1]/recover#_____________________w",
        &read(Seen::Recovery {
            server: "https://[2001:db8::1]".to_owned(),
            secret: [0xFF; 16],
        }),
    );
}

/// A code in the query string would reach the server's log, so it is no
/// claim link.
///
/// Verifies: SEC-MED-028, SEC-CLI-025
#[test]
fn replays_a_claim_code_in_the_query() {
    replay(
        "claim-code-in-the-query",
        b"https://music.example/claim?01234-56789-ABCDE-FGHJK-MNPQR-S9",
        &not_recognised(),
    );
}

/// The link names `music.example` but goes to `evil.example`.
///
/// Verifies: SEC-MED-028, SEC-CLI-025
#[test]
fn replays_credentials_before_the_host() {
    replay(
        "credentials-before-the-host",
        b"https://music.example@evil.example/invite#AAECAwQFBgcICQoLDA0ODw",
        &not_recognised(),
    );
}

/// The invalid octet becomes U+FFFD, which no secret holds.
///
/// Verifies: SEC-MED-028, SEC-CLI-025
#[test]
fn replays_invalid_utf8_in_the_secret() {
    replay(
        "invalid-utf8-in-the-secret",
        b"https://music.example/invite#AAECAwQFBgcICQoLDA0OD\xFF",
        &not_recognised(),
    );
}

/// The fourth letter of the host is the Cyrillic letter U+0456, which looks
/// like a Latin `i`.
///
/// Verifies: SEC-MED-028, SEC-CLI-025
#[test]
fn replays_a_lookalike_host() {
    replay(
        "lookalike-host",
        b"https://mus\xD1\x96c.example/invite#AAECAwQFBgcICQoLDA0ODw",
        &not_recognised(),
    );
}

/// A person may type a claim code in lower case; a server never writes one
/// so.
///
/// Verifies: SEC-MED-028, SEC-CLI-025
#[test]
fn replays_a_lower_case_claim_code() {
    replay(
        "lower-case-claim-code",
        b"https://music.example/claim#01234-56789-abcde-fghjk-mnpqr-s9",
        &not_recognised(),
    );
}

/// Padding would give the secret a second spelling.
///
/// Verifies: SEC-MED-028, SEC-CLI-025
#[test]
fn replays_a_padded_secret() {
    replay(
        "padded-secret",
        b"https://music.example/invite#AAECAwQFBgcICQoLDA0ODw==",
        &not_recognised(),
    );
}

/// Over plain HTTP a claim code may go to this machine and nowhere else.
///
/// Verifies: SEC-MED-028, SEC-CLI-025
#[test]
fn replays_plain_http_to_another_machine() {
    replay(
        "plain-http-to-another-machine",
        b"http://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
        &not_recognised(),
    );
}

/// A browser would open this as `https://music.example/claim`, but it is
/// not how a server or an address bar writes that link.
///
/// Verifies: SEC-MED-028, SEC-CLI-025
#[test]
fn replays_a_server_spelled_another_way() {
    replay(
        "server-spelled-another-way",
        b"HTTPS://Music.Example:443/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
        &not_recognised(),
    );
}

/// R1 issues no link to a page or an item.
///
/// Verifies: SEC-MED-028, SEC-CLI-025
#[test]
fn replays_an_unknown_path() {
    replay(
        "unknown-path",
        b"https://music.example/albums#AAECAwQFBgcICQoLDA0ODw",
        &not_recognised(),
    );
}
