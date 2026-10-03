//! The canary test SEC-OPS-013 asks for, set up here: a root secret with
//! known bytes, and a scan for those bytes in every form a leak could take
//! (raw, hexadecimal in either case, and base64 in either alphabet).
//!
//! Every later package that handles a secret extends it: it runs its own
//! output (log lines, HTTP bodies and headers, bundles, alert payloads)
//! through [`assert_no_canary`] with the canary loaded as its secret.

use std::fmt::Write as _;
use std::io::Read as _;

use gunmetal_core::base64::{self, Alphabet};
use gunmetal_fs::dataroot::{DataRoot, Policy};
use gunmetal_fs::host::HostFacts;
use gunmetal_fs::path::{DataDir, DataPath};
use gunmetal_secrets::random::OsRandom;
use gunmetal_secrets::root::{Root, SecretsError};
use gunmetal_testkit::tempdir::TempDir;

/// The canary root secret: 32 bytes no real secret will ever equal.
const CANARY: [u8; 32] = *b"gunmetal-canary-root-secret-0001";

/// Every form of `bytes` a leak could take.
fn forms(bytes: &[u8]) -> Vec<Vec<u8>> {
    let hex = bytes.iter().fold(String::new(), |mut out, byte| {
        write!(out, "{byte:02x}").unwrap();
        out
    });
    vec![
        bytes.to_vec(),
        hex.clone().into_bytes(),
        hex.to_uppercase().into_bytes(),
        base64::encode(bytes, Alphabet::Standard).into_bytes(),
        base64::encode(bytes, Alphabet::UrlSafe).into_bytes(),
    ]
}

/// Whether `haystack` holds `needle` anywhere.
fn holds(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// Fails when `output` holds the canary in any form.
fn assert_no_canary(output: &[u8]) {
    for form in forms(&CANARY) {
        assert!(!holds(output, &form));
    }
}

#[test]
fn the_scan_finds_the_canary_in_every_form() {
    for form in forms(&CANARY) {
        let mut output = b"prefix ".to_vec();
        output.extend_from_slice(&form);
        output.extend_from_slice(b" suffix");
        assert!(holds(&output, &form));
    }
    assert_eq!(
        forms(b"\x00\xff"),
        [
            b"\x00\xff".to_vec(),
            b"00ff".to_vec(),
            b"00FF".to_vec(),
            b"AP8=".to_vec(),
            b"AP8".to_vec()
        ]
    );
    assert_no_canary(b"Secret([redacted])");
}

#[test]
fn a_loaded_root_secret_formats_without_the_canary() {
    let dir = TempDir::new("secrets-canary").unwrap();
    let host = HostFacts::probe(dir.path()).unwrap();
    let data = DataRoot::open(dir.path(), &host, Policy::DEFAULT)
        .unwrap()
        .root;
    let root_key = DataPath::constant(DataDir::Secrets, "root.key");
    data.replace(&root_key, &CANARY).unwrap();
    let loaded: Result<Root, SecretsError> = Root::load_or_create(&data, &OsRandom);
    let mut stored = Vec::new();
    data.open_read(&root_key)
        .unwrap()
        .read_to_end(&mut stored)
        .unwrap();
    assert_eq!(stored, CANARY);
    let output = format!("{loaded:?} {loaded:#?}");
    assert_eq!(
        output,
        "Ok(Root { secret: Secret([redacted]) }) Ok(\n    Root {\n        secret: Secret([redacted]),\n    },\n)"
    );
    assert_no_canary(output.as_bytes());
}
